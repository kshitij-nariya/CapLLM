import { Skeleton } from "@/components/ui/skeleton"

export default function DashboardLoading() {
  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <Skeleton className="h-8 w-[250px] bg-zinc-900" />
        <Skeleton className="h-4 w-[350px] bg-zinc-900" />
      </div>

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        {[1, 2, 3, 4].map((i) => (
          <div key={i} className="rounded-xl border border-zinc-800 bg-zinc-900/50 p-6 space-y-3">
            <Skeleton className="h-4 w-24 bg-zinc-800" />
            <Skeleton className="h-8 w-20 bg-zinc-800" />
            <Skeleton className="h-3 w-32 bg-zinc-800" />
          </div>
        ))}
      </div>
      
      <Skeleton className="h-[400px] w-full rounded-xl bg-zinc-900" />
    </div>
  )
}
