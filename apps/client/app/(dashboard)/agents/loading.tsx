import { Skeleton } from "@/components/skeleton";

export default function AgentsLoading() {
  return (
    <>
      <div className="h-6 w-24 animate-pulse rounded bg-border/50" />
      <div className="mt-2 h-4 w-56 animate-pulse rounded bg-border/50" />
      <div className="mt-6 grid gap-4 sm:grid-cols-2">
        {Array.from({ length: 4 }).map((_, i) => (
          <div key={i} className="rounded-lg border border-border bg-card px-5 py-4">
            <div className="flex items-center justify-between">
              <div>
                <Skeleton className="h-5 w-24" />
                <Skeleton className="mt-1.5 h-3 w-16" />
              </div>
              <Skeleton className="h-4 w-14" />
            </div>
            <div className="mt-4">
              <Skeleton className="h-3 w-full" />
              <Skeleton className="mt-2 h-1.5 w-full rounded-full" />
            </div>
          </div>
        ))}
      </div>
    </>
  );
}
