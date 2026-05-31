"use client"

import * as React from "react"
import { Check, ChevronsUpDown, Building } from "lucide-react"
import { useSession } from "next-auth/react"

import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command"
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover"
interface OrgData {
  id: string
  name: string
  role: string
}

interface OrganizationSwitcherProps {
  organizations: OrgData[]
}

export function OrganizationSwitcher({ organizations }: OrganizationSwitcherProps) {
  const { data: session, update } = useSession()
  const [open, setOpen] = React.useState(false)

  const activeOrgId = session?.user?.organizationId
  const activeOrg = organizations.find((org) => org.id === activeOrgId)

  const handleSelect = async (org: OrgData) => {
    setOpen(false)
    if (org.id === activeOrgId) return

    await update({
      organizationId: org.id,
      role: org.role
    })
  }

  if (!activeOrg) return <div className="h-9 w-[200px] bg-zinc-900 rounded-md animate-pulse" />

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger render={<Button
          variant="outline"
          role="combobox"
          aria-expanded={open}
          className="w-[200px] justify-between border-zinc-800 bg-zinc-900/50 hover:bg-zinc-800 text-zinc-100"
        />}>
        <div className="flex items-center gap-2 truncate">
          <Building className="h-4 w-4 text-zinc-400" />
          <span className="truncate">{activeOrg.name}</span>
        </div>
        <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
      </PopoverTrigger>
      <PopoverContent className="w-[200px] p-0 border-zinc-800 bg-zinc-950 text-zinc-100">
        <Command className="bg-transparent">
          <CommandInput placeholder="Search organization..." className="h-9" />
          <CommandList>
            <CommandEmpty>No organization found.</CommandEmpty>
            <CommandGroup>
              {organizations.map((org) => (
                <CommandItem
                  key={org.id}
                  value={org.name}
                  onSelect={() => handleSelect(org)}
                  className="aria-selected:bg-zinc-800 data-[selected=true]:bg-zinc-800 cursor-pointer"
                >
                  <Check
                    className={cn(
                      "mr-2 h-4 w-4",
                      activeOrgId === org.id ? "opacity-100" : "opacity-0"
                    )}
                  />
                  {org.name}
                </CommandItem>
              ))}
            </CommandGroup>
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  )
}
