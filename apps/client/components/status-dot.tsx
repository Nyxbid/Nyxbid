const colors: Record<string, string> = {
  active: "bg-emerald-500",
  idle: "bg-zinc-400",
  error: "bg-rose-500",
  confirmed: "bg-emerald-500",
  pending: "bg-amber-500",
  failed: "bg-rose-500",
};

interface StatusDotProps {
  status: string;
}

export function StatusDot({ status }: StatusDotProps) {
  const color = colors[status] ?? "bg-zinc-400";
  return (
    <span className="inline-flex items-center gap-1.5 text-xs capitalize text-muted">
      <span className={`inline-block h-1.5 w-1.5 rounded-full ${color}`} />
      {status}
    </span>
  );
}
