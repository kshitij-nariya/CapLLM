import { auth } from "@/auth"
import prisma from "@/lib/db"
import { CreateKeyForm } from "./create-key-form"
import { redirect } from "next/navigation"
import { Badge } from "@/components/ui/badge"

export default async function KeysPage() {
  const session = await auth()
  if (!session?.user?.organizationId) redirect("/login")
  
  const keys = await prisma.virtualKey.findMany({
    where: { organizationId: session.user.organizationId },
    orderBy: { createdAt: "desc" }
  })

  return (
    <div className="max-w-5xl space-y-8">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Virtual Keys</h1>
        <p className="text-zinc-400">Generate and manage virtual keys for your AI workloads.</p>
      </div>

      <div className="p-6 rounded-xl border border-zinc-800 bg-zinc-950">
        <h2 className="text-xl font-semibold mb-4 text-zinc-100">Create New Key</h2>
        <CreateKeyForm />
      </div>

      <div className="rounded-xl border border-zinc-800 bg-zinc-950">
        <div className="p-6">
          <h2 className="text-xl font-semibold mb-4 text-zinc-100">Active Keys</h2>
          
          {keys.length === 0 ? (
            <p className="text-zinc-500">No active keys found.</p>
          ) : (
            <div className="space-y-4">
              {keys.map(k => (
                <div key={k.id} className="flex items-center justify-between p-4 rounded-lg border border-zinc-800 bg-zinc-900/50">
                  <div>
                    <p className="font-medium text-zinc-200">{k.name}</p>
                    <div className="flex items-center gap-2 mt-1">
                      <code className="text-sm text-zinc-400">{k.prefix}••••••••</code>
                      <Badge variant="outline" className="bg-emerald-500/10 text-emerald-400 border-emerald-500/20">Active</Badge>
                    </div>
                  </div>
                  <div className="text-sm text-zinc-500 text-right">
                    <p>Created: {k.createdAt.toLocaleDateString()}</p>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
