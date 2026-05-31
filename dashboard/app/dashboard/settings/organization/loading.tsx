import { Skeleton } from "@/components/ui/skeleton"

export default function OrganizationSettingsLoading() {
  return (
    <div className="max-w-5xl space-y-8">
      <div className="flex items-center justify-between">
        <div className="space-y-2">
          <Skeleton className="h-8 w-64 bg-zinc-900" />
          <Skeleton className="h-4 w-48 bg-zinc-900" />
        </div>
        <Skeleton className="h-10 w-32 bg-zinc-900" />
      </div>

      <div className="space-y-4">
        <div className="flex gap-2">
          <Skeleton className="h-10 w-24 bg-zinc-900" />
          <Skeleton className="h-10 w-32 bg-zinc-900" />
        </div>
        <Skeleton className="h-[400px] w-full bg-zinc-900 rounded-md" />
      </div>
    </div>
  )
}
