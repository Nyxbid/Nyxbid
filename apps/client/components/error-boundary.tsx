"use client";

import { Component, type ReactNode } from "react";

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false };

  static getDerivedStateFromError(): State {
    return { hasError: true };
  }

  render() {
    if (this.state.hasError) {
      return (
        this.props.fallback ?? (
          <div className="flex min-h-[200px] items-center justify-center rounded-lg border border-border bg-card">
            <div className="text-center">
              <p className="text-sm font-medium text-foreground">
                Something went wrong
              </p>
              <p className="mt-1 text-xs text-muted">
                Try refreshing the page.
              </p>
              <button
                onClick={() => this.setState({ hasError: false })}
                className="mt-3 rounded-md bg-accent px-4 py-1.5 text-xs font-medium text-white transition-colors hover:bg-accent/90"
              >
                Retry
              </button>
            </div>
          </div>
        )
      );
    }

    return this.props.children;
  }
}
