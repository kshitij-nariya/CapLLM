import { auth } from "@/auth"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar"
import { updateProfile } from "@/actions/profile"

export default async function ProfileSettingsPage() {
  const session = await auth()
  const user = session?.user

  return (
    <div className="max-w-2xl space-y-8">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Profile Settings</h1>
        <p className="text-zinc-400">Manage your personal account preferences.</p>
      </div>

      <div className="space-y-6">
        <div className="flex items-center gap-6">
          <Avatar className="h-24 w-24 border border-zinc-800">
            <AvatarImage src={user?.image || ""} />
            <AvatarFallback className="bg-zinc-800 text-3xl text-zinc-300">
              {user?.name?.charAt(0) || "U"}
            </AvatarFallback>
          </Avatar>
          <div className="space-y-2">
            <Button variant="outline" className="border-zinc-800 bg-zinc-900/50 text-zinc-100 hover:bg-zinc-800">
              Upload new avatar
            </Button>
            <p className="text-xs text-zinc-500">JPG, GIF or PNG. 1MB max.</p>
          </div>
        </div>

        <form action={updateProfile} className="space-y-4 border-t border-zinc-800 pt-6">
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="name">Full Name</Label>
              <Input 
                id="name" 
                name="name"
                defaultValue={user?.name || ""} 
                className="bg-zinc-900/50 border-zinc-800 text-zinc-100" 
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="email">Email Address</Label>
              <Input 
                id="email" 
                disabled 
                defaultValue={user?.email || ""} 
                className="bg-zinc-900/50 border-zinc-800 text-zinc-500" 
              />
            </div>
          </div>
          
          <div className="space-y-2 pt-4">
            <Button className="bg-indigo-600 hover:bg-indigo-700 text-white">
              Save Changes
            </Button>
          </div>
        </form>
      </div>
    </div>
  )
}
