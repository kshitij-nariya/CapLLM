"use client"

import * as React from "react"
import { createVirtualKey } from "@/actions/keys"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"

export function CreateKeyForm() {
  const [rawKey, setRawKey] = React.useState<string | null>(null)
  const [loading, setLoading] = React.useState(false)

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault()
    setLoading(true)
    setRawKey(null)
    const formData = new FormData(e.currentTarget)
    try {
      const res = await createVirtualKey(formData)
      if (res.success && res.rawKey) {
        setRawKey(res.rawKey)
      }
    } catch (err) {
      console.error(err)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="space-y-4">
      <form onSubmit={handleSubmit} className="flex items-end gap-4">
        <div className="flex-1 space-y-2">
          <label className="text-sm text-zinc-400">Key Name</label>
          <Input name="name" placeholder="Production Key" required className="bg-zinc-900/50 border-zinc-800 text-zinc-100" />
        </div>
        <Button type="submit" disabled={loading} className="bg-indigo-600 text-white hover:bg-indigo-700">
          {loading ? "Generating..." : "Generate Key"}
        </Button>
      </form>

      {rawKey && (
        <div className="p-4 bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 rounded-md">
          <p className="font-semibold mb-2">Key generated successfully!</p>
          <p className="text-sm mb-4">Please copy this key now. You will not be able to see it again.</p>
          <code className="bg-black/40 px-3 py-2 rounded-md block select-all font-mono">{rawKey}</code>
        </div>
      )}
    </div>
  )
}
