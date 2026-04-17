export const dynamic = "force-dynamic";

export default function QuotesPage() {
  return (
    <>
      <h1 className="text-xl font-semibold tracking-tight">Quotes</h1>
      <p className="mt-1 text-sm text-muted">
        Sealed commitments from makers, scoped to an intent.
      </p>

      <div className="mt-6 rounded-lg border border-border bg-card px-5 py-8 text-center text-sm text-muted">
        Pick an intent from the Intents page to view its quote stream.
      </div>
    </>
  );
}
