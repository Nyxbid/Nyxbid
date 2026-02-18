import { Skeleton } from "@/components/skeleton";

export default function Loading() {
  return (
    <>
      <Skeleton className="h-6 w-32" />
      <Skeleton className="mt-2 h-4 w-72" />
      <Skeleton className="mt-6 h-40" />
    </>
  );
}
