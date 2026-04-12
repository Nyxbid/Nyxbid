import { Skeleton } from "@/components/skeleton";

export default function PoliciesLoading() {
  return (
    <>
      <div className="h-6 w-24 animate-pulse rounded bg-border/50" />
      <div className="mt-2 h-4 w-56 animate-pulse rounded bg-border/50" />
      <div className="mt-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {Array.from({ length: 3 }).map((_, i) => (
          <div key={i} className="rounded-lg border border-border bg-card px-5 py-4">
            <div className="flex items-center justify-between">
              <Skeleton className="h-5 w-32" />
              <Skeleton className="h-5 w-14 rounded-full" />
            </div>
            <div className="mt-4 space-y-2">
              <Skeleton className="h-4 w-full" />
              <Skeleton className="h-4 w-full" />
            </div>
            <div className="mt-4 flex gap-1.5">
              <Skeleton className="h-5 w-16 rounded" />
              <Skeleton className="h-5 w-20 rounded" />
            </div>
          </div>
        ))}
      </div>
    </>
  );
}
