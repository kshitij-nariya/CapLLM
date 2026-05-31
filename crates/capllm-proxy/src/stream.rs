//! Zero-copy SSE parser and stream transformer.
//!
//! Consumes the raw byte stream from an upstream provider, parses SSE frames
//! without unnecessary allocations, and translates each event into an
//! OpenAI-compatible [`ChatCompletionChunk`] for re-emission.

use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::{Bytes, BytesMut};
use capllm_core::types::gen_completion_id;
use capllm_core::{ChatCompletionChunk, GatewayError, Provider};
use capllm_translate::TranslationEngine;
use futures::stream::Stream;


/// A parsed SSE frame.
#[derive(Debug)]
struct SseFrame {
    /// The `event:` field (empty string if absent).
    event: String,
    /// The `data:` field content.
    data: String,
}

/// Stateful SSE line parser that handles chunk boundaries correctly.
///
/// SSE data can arrive in arbitrary byte boundaries from the network. This
/// parser buffers partial lines and yields complete [`SseFrame`]s only when
/// a blank line delimiter is encountered.
struct SseLineParser {
    buffer: BytesMut,
    current_event: String,
    current_data: String,
}

impl SseLineParser {
    fn new() -> Self {
        Self {
            buffer: BytesMut::with_capacity(4096),
            current_event: String::new(),
            current_data: String::new(),
        }
    }

    /// Feed a chunk of bytes and extract any complete SSE frames.
    fn feed(&mut self, chunk: &Bytes) -> Vec<SseFrame> {
        self.buffer.extend_from_slice(chunk);
        let mut frames = Vec::new();

        loop {
            // Find the next newline in the buffer
            let newline_pos = self.buffer.iter().position(|&b| b == b'\n');
            let Some(pos) = newline_pos else {
                break;
            };

            // Extract the line (excluding the newline)
            let line_bytes = self.buffer.split_to(pos + 1);
            let line = std::str::from_utf8(&line_bytes)
                .unwrap_or("")
                .trim_end_matches(['\r', '\n']);

            if line.is_empty() {
                // Blank line = end of SSE frame
                if !self.current_data.is_empty() {
                    frames.push(SseFrame {
                        event: std::mem::take(&mut self.current_event),
                        data: std::mem::take(&mut self.current_data),
                    });
                }
            } else if let Some(value) = line.strip_prefix("event:") {
                self.current_event.clear();
                self.current_event.push_str(value.trim_start());
            } else if let Some(value) = line.strip_prefix("data:") {
                let value = value.trim_start();
                if !self.current_data.is_empty() {
                    self.current_data.push('\n');
                }
                self.current_data.push_str(value);
            }
            // Lines starting with ':' are comments — silently skip
        }

        frames
    }
}

/// A stream adapter that converts a raw byte stream from an upstream provider
/// into a stream of OpenAI-formatted [`ChatCompletionChunk`]s.
pub struct OpenAiSseStream {
    inner: Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>,
    parser: SseLineParser,
    provider: Provider,
    completion_id: String,
    model: String,
    done: bool,
    /// Buffer of translated chunks waiting to be yielded.
    pending: Vec<ChatCompletionChunk>,
}

impl OpenAiSseStream {
    fn new(
        inner: Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>,
        provider: Provider,
        model: String,
    ) -> Self {
        Self {
            inner,
            parser: SseLineParser::new(),
            provider,
            completion_id: gen_completion_id(),
            model,
            done: false,
            pending: Vec::new(),
        }
    }
}

impl Stream for OpenAiSseStream {
    type Item = Result<ChatCompletionChunk, GatewayError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        // Yield any buffered chunks first
        if !this.pending.is_empty() {
            return Poll::Ready(Some(Ok(this.pending.remove(0))));
        }

        if this.done {
            return Poll::Ready(None);
        }

        // Poll the inner byte stream
        match Pin::new(&mut this.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                let frames = this.parser.feed(&chunk);

                for frame in frames {
                    // Check for end-of-stream signals
                    if frame.data == "[DONE]" {
                        this.done = true;
                        continue;
                    }

                    if TranslationEngine::is_stream_done(
                        this.provider,
                        &frame.event,
                        &frame.data,
                    ) {
                        this.done = true;
                    }

                    match TranslationEngine::translate_sse_event(
                        this.provider,
                        &frame.event,
                        &frame.data,
                        &this.completion_id,
                        &this.model,
                    ) {
                        Ok(Some(translated)) => this.pending.push(translated),
                        Ok(None) => {} // Skip non-content events
                        Err(e) => return Poll::Ready(Some(Err(e))),
                    }
                }

                if this.pending.is_empty() {
                    // No frames extracted yet, need more data — re-register waker
                    cx.waker().wake_by_ref();
                    Poll::Pending
                } else {
                    Poll::Ready(Some(Ok(this.pending.remove(0))))
                }
            }
            Poll::Ready(Some(Err(e))) => {
                Poll::Ready(Some(Err(GatewayError::HttpClient(e))))
            }
            Poll::Ready(None) => {
                this.done = true;
                if this.pending.is_empty() {
                    Poll::Ready(None)
                } else {
                    Poll::Ready(Some(Ok(this.pending.remove(0))))
                }
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Convert an upstream provider response into an OpenAI-compatible SSE chunk
/// stream.
///
/// This is the main entry-point used by the server handler. The returned
/// stream is directly compatible with Axum's `Sse` response type.
pub fn into_openai_sse_stream(
    response: reqwest::Response,
    provider: Provider,
    model: String,
) -> impl Stream<Item = Result<ChatCompletionChunk, GatewayError>> + Send {
    let byte_stream = Box::pin(response.bytes_stream());
    OpenAiSseStream::new(byte_stream, provider, model)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sse_parser_basic() {
        let mut parser = SseLineParser::new();
        let chunk = Bytes::from("event: message\ndata: {\"text\":\"hello\"}\n\n");
        let frames = parser.feed(&chunk);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].event, "message");
        assert_eq!(frames[0].data, "{\"text\":\"hello\"}");
    }

    #[test]
    fn sse_parser_split_across_chunks() {
        let mut parser = SseLineParser::new();

        // First chunk ends mid-line
        let frames1 = parser.feed(&Bytes::from("event: cont"));
        assert!(frames1.is_empty());

        // Second chunk completes the frame
        let frames2 = parser.feed(&Bytes::from(
            "ent_block_delta\ndata: {\"delta\":\"hi\"}\n\n",
        ));
        assert_eq!(frames2.len(), 1);
        assert_eq!(frames2[0].event, "content_block_delta");
    }

    #[test]
    fn sse_parser_multiple_frames() {
        let mut parser = SseLineParser::new();
        let chunk = Bytes::from("data: first\n\ndata: second\n\n");
        let frames = parser.feed(&chunk);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].data, "first");
        assert_eq!(frames[1].data, "second");
    }

    #[test]
    fn sse_parser_skips_comments() {
        let mut parser = SseLineParser::new();
        let chunk = Bytes::from(": this is a comment\ndata: actual\n\n");
        let frames = parser.feed(&chunk);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].data, "actual");
    }
}
