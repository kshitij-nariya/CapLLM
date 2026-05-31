import { auth } from "@/auth"
import { redirect } from "next/navigation"
import prisma from "@/lib/db"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { MembersTable } from "./members-table"
import { InviteModal } from "./invite-modal"

export default async function OrganizationSettingsPage() {
  const session = await auth()
  const user = session?.user

  if (!user || user.role === "Employee") {
    redirect("/dashboard")
  }

  const org = await prisma.organization.findUnique({
    where: { id: user.organizationId }
  })

  const memberships = await prisma.organizationMember.findMany({
    where: { organizationId: user.organizationId },
    include: { user: true }
  })

  const members = memberships.map(m => ({
    id: m.userId,
    name: m.user.name || "Unknown",
    email: m.user.email || "",
    role: m.role,
    avatar: m.user.image || `https://api.dicebear.com/7.x/avataaars/svg?seed=${m.userId}`
  }))

  return (
    <div className="max-w-5xl space-y-8">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Organization Settings</h1>
          <p className="text-zinc-400">Manage settings for {org?.name}</p>
        </div>
        
        <InviteModal />
      </div>

      <Tabs defaultValue="members" className="w-full">
        <TabsList className="bg-zinc-900 border border-zinc-800">
          <TabsTrigger value="members" className="data-[state=active]:bg-zinc-800 data-[state=active]:text-zinc-100">
            Members
          </TabsTrigger>
          <TabsTrigger value="budgets" className="data-[state=active]:bg-zinc-800 data-[state=active]:text-zinc-100">
            FinOps Budgets
          </TabsTrigger>
          <TabsTrigger value="billing" className="data-[state=active]:bg-zinc-800 data-[state=active]:text-zinc-100" disabled={user.role !== 'Org_Owner'}>
            Billing
          </TabsTrigger>
        </TabsList>
        <TabsContent value="members" className="mt-6">
          <div className="rounded-md border border-zinc-800 bg-zinc-950">
            <MembersTable members={members} currentUserRole={user.role} currentUserId={user.id || ""} />
          </div>
        </TabsContent>
        <TabsContent value="budgets" className="mt-6">
          <div className="rounded-xl border border-zinc-800 bg-zinc-900/50 p-8 text-center text-zinc-500">
            Configure hard token limits and spend caps per employee/team.
          </div>
        </TabsContent>
        <TabsContent value="billing" className="mt-6">
          <div className="rounded-xl border border-zinc-800 bg-zinc-900/50 p-8 text-center text-zinc-500">
            Billing management. Only visible to Org Owners.
          </div>
        </TabsContent>
      </Tabs>
    </div>
  )
}
