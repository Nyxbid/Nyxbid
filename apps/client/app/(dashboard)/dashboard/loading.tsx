import { StatCardSkeleton, TableSkeleton } from "@/components/skeleton";

export default function DashboardLoading() {
  return (
    <>
      <div className="h-6 w-32 animate-pulse rounded bg-border/50" />
      <div className="mt-2 h-4 w-64 animate-pulse rounded bg-border/50" />

      <div className="mt-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <StatCardSkeleton />
        <StatCardSkeleton />
        <StatCardSkeleton />
        <StatCardSkeleton />
      </div>

      <div className="mt-10">
        <div className="h-4 w-28 animate-pulse rounded bg-border/50" />
        <div className="mt-3">
          <TableSkeleton rows={5} cols={5} />
        </div>
      </div>
    </>
  );
}
