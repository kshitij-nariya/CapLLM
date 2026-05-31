"use client"

import Link from "next/link"
import { usePathname } from "next/navigation"
import { 
  BarChart3, 
  Key, 
  Settings, 
  ShieldAlert, 
  Users, 
  Hexagon,
  CreditCard
} from "lucide-react"

import { cn } from "@/lib/utils"
import { RoleGuard } from "./role-guard"
export type Role = 'Org_Owner' | 'Org_Admin' | 'Employee'

interface SidebarProps {
  userRole: Role
}

export function Sidebar({ userRole }: SidebarProps) {
  const pathname = usePathname()

  const routes = [
    {
      label: "Overview",
      icon: BarChart3,
      href: "/dashboard",
      roles: ["Org_Owner", "Org_Admin", "Employee"],
    },
    {
      label: "Virtual Keys",
      icon: Key,
      href: "/dashboard/keys",
      roles: ["Org_Owner", "Org_Admin", "Employee"],
    },
    {
      label: "Guardrails & DLP",
      icon: ShieldAlert,
      href: "/dashboard/guardrails",
      roles: ["Org_Owner", "Org_Admin"],
    },
    {
      label: "Team Members",
      icon: Users,
      href: "/dashboard/settings/organization",
      roles: ["Org_Owner", "Org_Admin"],
    },
    {
      label: "Billing",
      icon: CreditCard,
      href: "/dashboard/billing",
      roles: ["Org_Owner"],
    },
    {
      label: "Settings",
      icon: Settings,
      href: "/dashboard/settings/profile",
      roles: ["Org_Owner", "Org_Admin", "Employee"],
    },
  ]

  return (
    <div className="flex w-64 flex-col border-r border-zinc-800 bg-zinc-950/50">
      <div className="flex h-16 items-center px-6 border-b border-zinc-800">
        <Link href="/dashboard" className="flex items-center gap-2 font-bold tracking-tight text-indigo-400">
          <Hexagon className="h-6 w-6" />
          <span>CapLLM</span>
        </Link>
      </div>
      <div className="flex-1 py-6 overflow-y-auto">
        <nav className="space-y-1 px-4">
          {routes.map((route) => {
            const isActive = pathname === route.href || pathname.startsWith(`${route.href}/`)
            
            return (
              <RoleGuard key={route.href} allowedRoles={route.roles as Role[]}>
                <Link
                  href={route.href}
                  className={cn(
                    "flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
                    isActive 
                      ? "bg-indigo-500/10 text-indigo-400" 
                      : "text-zinc-400 hover:bg-zinc-900/80 hover:text-zinc-100"
                  )}
                >
                  <route.icon className={cn("h-4 w-4", isActive ? "text-indigo-400" : "text-zinc-500")} />
                  {route.label}
                </Link>
              </RoleGuard>
            )
          })}
        </nav>
      </div>
    </div>
  )
}
