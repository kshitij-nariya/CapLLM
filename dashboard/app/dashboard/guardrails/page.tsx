import { auth } from "@/auth"

export default async function GuardrailsPage() {
  const session = await auth()
  
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Guardrails & DLP</h1>
        <p className="text-zinc-400">Configure data loss prevention rules and prompt injection guardrails.</p>
      </div>

      <div className="rounded-xl border border-zinc-800 bg-zinc-900/50 p-8 text-center text-zinc-500">
        Guardrails & DLP configuration page placeholder.
      </div>
    </div>
  )
}
