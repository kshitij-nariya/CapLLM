"use client"

import * as React from "react"
import { Plus } from "lucide-react"

import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"

export function InviteModal() {
  const [open, setOpen] = React.useState(false)
  const [email, setEmail] = React.useState("")
  const [role, setRole] = React.useState("Employee")
  const [loading, setLoading] = React.useState(false)

  const handleInvite = async () => {
    setLoading(true)
    try {
      const formData = new FormData()
      formData.append("email", email)
      formData.append("role", role)
      
      const { inviteMember } = await import('@/actions/organization')
      await inviteMember(formData)
      setOpen(false)
      setEmail("")
      setRole("Employee")
    } catch (e) {
      console.error(e)
    } finally {
      setLoading(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger render={<Button className="bg-indigo-600 hover:bg-indigo-700 text-white" />}>
        <Plus className="mr-2 h-4 w-4" />
        Invite Member
      </DialogTrigger>
      <DialogContent className="sm:max-w-[425px] border-zinc-800 bg-zinc-950 text-zinc-100">
        <DialogHeader>
          <DialogTitle>Invite Member</DialogTitle>
          <DialogDescription className="text-zinc-400">
            Send an email invitation to join this organization.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 py-4">
          <div className="grid grid-cols-4 items-center gap-4">
            <Label htmlFor="email" className="text-right">
              Email
            </Label>
            <Input
              id="email"
              placeholder="colleague@example.com"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              className="col-span-3 bg-zinc-900/50 border-zinc-800 text-zinc-100"
            />
          </div>
          <div className="grid grid-cols-4 items-center gap-4">
            <Label htmlFor="role" className="text-right text-zinc-400">
              Role
            </Label>
            <div className="col-span-3">
              <Select value={role} onValueChange={(val) => val && setRole(val)}>
                <SelectTrigger className="bg-zinc-900/50 border-zinc-800 text-zinc-100">
                  <SelectValue placeholder="Select a role" />
                </SelectTrigger>
                <SelectContent className="bg-zinc-950 border-zinc-800 text-zinc-100">
                  <SelectItem value="Org_Owner" className="focus:bg-zinc-800">Organization Owner</SelectItem>
                  <SelectItem value="Org_Admin" className="focus:bg-zinc-800">Organization Admin</SelectItem>
                  <SelectItem value="Employee" className="focus:bg-zinc-800">Employee</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" className="border-zinc-800 bg-zinc-900/50 hover:bg-zinc-800 text-zinc-100" onClick={() => setOpen(false)} disabled={loading}>
            Cancel
          </Button>
          <Button type="submit" className="bg-indigo-600 hover:bg-indigo-700 text-white" onClick={handleInvite} disabled={loading || !email}>
            {loading ? "Inviting..." : "Send Invite"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
