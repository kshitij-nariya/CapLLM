import { auth } from "@/auth"
import { redirect } from "next/navigation"
import { OrganizationSwitcher } from "@/components/organization-switcher"
import { UserMenu } from "@/components/user-menu"
import { Sidebar } from "@/components/sidebar"
import prisma from "@/lib/db"

export default async function DashboardLayout({
  children,
}: {
  children: React.ReactNode
}) {
  const session = await auth()
  
  if (!session?.user?.id) {
    redirect("/login")
  }

  const userMemberships = await prisma.organizationMember.findMany({
    where: { userId: session.user.id },
    include: { organization: true }
  })
  
  const organizations = userMemberships.map(m => ({
    id: m.organization.id,
    name: m.organization.name,
    role: m.role
  }))

  return (
    <div className="flex min-h-screen bg-zinc-950 text-zinc-50">
      <Sidebar userRole={session.user.role} />
      
      <div className="flex flex-col flex-1">
        <header className="sticky top-0 z-30 flex h-16 items-center justify-between border-b border-zinc-800 bg-zinc-950/80 px-6 backdrop-blur-md">
          <div className="flex items-center gap-4">
            <OrganizationSwitcher organizations={organizations} />
          </div>
          <div className="flex items-center gap-4">
            <UserMenu user={session.user} />
          </div>
        </header>

        <main className="flex-1 overflow-auto p-6">
          {children}
        </main>
      </div>
    </div>
  )
}
