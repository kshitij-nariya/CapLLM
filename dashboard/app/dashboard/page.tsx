import { auth } from "@/auth"

export default async function DashboardPage() {
  const session = await auth()
  
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Overview</h1>
        <p className="text-zinc-400">Welcome back, {session?.user?.name}. Here is your proxy traffic summary.</p>
      </div>

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <div className="rounded-xl border border-zinc-800 bg-zinc-900/50 p-6">
          <div className="text-sm font-medium text-zinc-400">Total Requests</div>
          <div className="mt-2 text-3xl font-bold">12,345</div>
          <div className="mt-1 text-xs text-emerald-400">+14% from last month</div>
        </div>
        <div className="rounded-xl border border-zinc-800 bg-zinc-900/50 p-6">
          <div className="text-sm font-medium text-zinc-400">Tokens Processed</div>
          <div className="mt-2 text-3xl font-bold">4.2M</div>
          <div className="mt-1 text-xs text-emerald-400">+8% from last month</div>
        </div>
        <div className="rounded-xl border border-zinc-800 bg-zinc-900/50 p-6">
          <div className="text-sm font-medium text-zinc-400">Avg Latency</div>
          <div className="mt-2 text-3xl font-bold">1.2ms</div>
          <div className="mt-1 text-xs text-emerald-400">-0.4ms from last month</div>
        </div>
        <div className="rounded-xl border border-zinc-800 bg-zinc-900/50 p-6">
          <div className="text-sm font-medium text-zinc-400">DLP Masked Entities</div>
          <div className="mt-2 text-3xl font-bold">892</div>
          <div className="mt-1 text-xs text-rose-400">+42 from last month</div>
        </div>
      </div>
      
      {/* Placeholder for charts */}
      <div className="h-[400px] rounded-xl border border-zinc-800 bg-zinc-900/30 flex items-center justify-center text-zinc-500">
        Activity Chart Visualization
      </div>
    </div>
  )
}
