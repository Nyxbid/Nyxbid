import { Skeleton } from "@/components/skeleton";

export default function Loading() {
  return (
    <>
      <Skeleton className="h-6 w-40" />
      <Skeleton className="mt-2 h-4 w-72" />
      <div className="mt-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {Array.from({ length: 4 }).map((_, i) => (
          <Skeleton key={i} className="h-24" />
        ))}
      </div>
    </>
  );
}
