"use client"

import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar"
export type Role = 'Org_Owner' | 'Org_Admin' | 'Employee'

export interface UserMember {
  id: string
  name: string
  email: string
  role: string
  avatar?: string
  image?: string
}

interface MembersTableProps {
  members: UserMember[]
  currentUserRole: string
  currentUserId: string
}

export function MembersTable({ members, currentUserRole, currentUserId }: MembersTableProps) {
  const canManage = (targetRole: string) => {
    if (currentUserRole === 'Org_Owner') return true
    if (currentUserRole === 'Org_Admin' && targetRole === 'Employee') return true
    return false
  }

  const handleRevoke = async (id: string) => {
    try {
      const { revokeAccess } = await import('@/actions/organization')
      await revokeAccess(id)
    } catch (e) {
      console.error(e)
    }
  }

  return (
    <Table>
      <TableHeader className="bg-zinc-900/50">
        <TableRow className="border-zinc-800 hover:bg-zinc-900/50">
          <TableHead className="text-zinc-400">Member</TableHead>
          <TableHead className="text-zinc-400">Role</TableHead>
          <TableHead className="text-right text-zinc-400">Actions</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {members.map((member) => (
          <TableRow key={member.id} className="border-zinc-800 hover:bg-zinc-900/50">
            <TableCell>
              <div className="flex items-center gap-3">
                <Avatar className="h-9 w-9 border border-zinc-800">
                  <AvatarImage src={member.image} />
                  <AvatarFallback className="bg-zinc-800 text-zinc-300">
                    {member.name.charAt(0)}
                  </AvatarFallback>
                </Avatar>
                <div>
                  <div className="font-medium text-zinc-200">
                    {member.name}
                    {member.id === currentUserId && <span className="ml-2 text-xs text-zinc-500">(You)</span>}
                  </div>
                  <div className="text-xs text-zinc-500">{member.email}</div>
                </div>
              </div>
            </TableCell>
            <TableCell>
              <Badge 
                variant="outline" 
                className={
                  member.role === 'Org_Owner' ? 'border-purple-500/30 text-purple-400 bg-purple-500/10' :
                  member.role === 'Org_Admin' ? 'border-indigo-500/30 text-indigo-400 bg-indigo-500/10' :
                  'border-zinc-700 text-zinc-400 bg-zinc-800/50'
                }
              >
                {member.role.replace('_', ' ')}
              </Badge>
            </TableCell>
            <TableCell className="text-right">
              <Button
                variant="destructive"
                size="sm"
                className="bg-red-500/10 text-red-500 hover:bg-red-500/20 border border-red-500/20"
                disabled={member.id === currentUserId || !canManage(member.role)}
                onClick={() => handleRevoke(member.id)}
              >
                Revoke Access
              </Button>
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  )
}
