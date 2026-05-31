import { auth } from "@/auth"

export default async function BillingPage() {
  const session = await auth()
  
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Billing & FinOps</h1>
        <p className="text-zinc-400">Manage your subscription, view invoices, and track organizational AI spend.</p>
      </div>

      <div className="rounded-xl border border-zinc-800 bg-zinc-900/50 p-8 text-center text-zinc-500">
        Billing configuration page placeholder.
      </div>
    </div>
  )
}
