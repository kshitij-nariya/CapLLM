"use client"

import { LogOut, Settings, User as UserIcon } from "lucide-react"
import { signOut } from "next-auth/react"
import { DefaultSession } from "next-auth"

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar"
import Link from "next/link"

interface UserMenuProps {
  user: DefaultSession["user"] & { role: string }
}

export function UserMenu({ user }: UserMenuProps) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger className="flex items-center gap-2 outline-none rounded-full ring-offset-zinc-950 focus-visible:ring-2 focus-visible:ring-indigo-500">
        <Avatar className="h-8 w-8 border border-zinc-800">
          <AvatarImage src={user.image || ""} alt={user.name || "User"} />
          <AvatarFallback className="bg-zinc-800 text-zinc-300">
            {user.name?.charAt(0) || "U"}
          </AvatarFallback>
        </Avatar>
      </DropdownMenuTrigger>
      <DropdownMenuContent className="w-56 border-zinc-800 bg-zinc-950 text-zinc-100" align="end">
        <DropdownMenuLabel className="font-normal">
          <div className="flex flex-col space-y-1">
            <p className="text-sm font-medium leading-none">{user.name}</p>
            <p className="text-xs leading-none text-zinc-500">
              {user.email}
            </p>
          </div>
        </DropdownMenuLabel>
        <DropdownMenuSeparator className="bg-zinc-800" />
        <DropdownMenuGroup>
          <DropdownMenuItem render={<Link href="/dashboard/settings/profile" />} className="focus:bg-zinc-800 cursor-pointer">
            <UserIcon className="mr-2 h-4 w-4" />
            <span>Profile Settings</span>
          </DropdownMenuItem>
          {['Org_Owner', 'Org_Admin'].includes(user.role) && (
            <DropdownMenuItem render={<Link href="/dashboard/settings/organization" />} className="focus:bg-zinc-800 cursor-pointer">
              <Settings className="mr-2 h-4 w-4" />
              <span>Organization Settings</span>
            </DropdownMenuItem>
          )}
        </DropdownMenuGroup>
        <DropdownMenuSeparator className="bg-zinc-800" />
        <DropdownMenuItem 
          onClick={() => signOut({ callbackUrl: "/login" })}
          className="text-red-400 focus:bg-red-950 focus:text-red-300 cursor-pointer"
        >
          <LogOut className="mr-2 h-4 w-4" />
          <span>Log out</span>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
