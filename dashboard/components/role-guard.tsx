"use client"

import { useSession } from "next-auth/react"
export type Role = 'Org_Owner' | 'Org_Admin' | 'Employee'

interface RoleGuardProps {
  allowedRoles: Role[]
  children: React.ReactNode
  fallback?: React.ReactNode
}

export function RoleGuard({ allowedRoles, children, fallback = null }: RoleGuardProps) {
  const { data: session, status } = useSession()

  if (status === "loading") {
    // Optionally return a skeleton here if preferred, but usually returning null is fine
    // because the parent page handles loading state or we don't want to show flashes.
    return null
  }

  if (!session?.user?.role) {
    return <>{fallback}</>
  }

  if (allowedRoles.includes(session.user.role)) {
    return <>{children}</>
  }

  return <>{fallback}</>
}
