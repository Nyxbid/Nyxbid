import { TableSkeleton } from "@/components/skeleton";

export default function ReceiptsLoading() {
  return (
    <>
      <div className="h-6 w-28 animate-pulse rounded bg-border/50" />
      <div className="mt-2 h-4 w-64 animate-pulse rounded bg-border/50" />
      <div className="mt-6">
        <TableSkeleton rows={6} cols={6} />
      </div>
    </>
  );
}
