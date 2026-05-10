import { Skeleton } from "@/components/skeleton";

export default function Loading() {
  return (
    <>
      <Skeleton className="h-3 w-24" />
      <Skeleton className="mt-6 h-28" />
      <Skeleton className="mt-4 h-20" />
      <div className="mt-6 grid gap-6 lg:grid-cols-[1fr_360px]">
        <Skeleton className="h-64" />
        <div className="space-y-4">
          <Skeleton className="h-44" />
          <Skeleton className="h-44" />
        </div>
      </div>
    </>
  );
}
