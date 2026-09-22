import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

class ErrorBoundary extends React.Component<{ children: React.ReactNode }, { error: Error | null }> {
  state = { error: null as Error | null };
  static getDerivedStateFromError(error: Error) {
    return { error };
  }
  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error("render error", error, info.componentStack);
  }
  render() {
    if (!this.state.error) return this.props.children;
    return (
      <div className="flex h-full flex-col gap-3 bg-bg p-6 text-fg">
        <div className="text-lg font-semibold">FinCompass hit a render error</div>
        <pre className="overflow-auto rounded-md border border-critical/40 bg-panel p-3 text-xs text-critical">{String(this.state.error)}</pre>
        <button className="w-fit rounded-md bg-accent px-3 py-1.5 text-sm text-white" onClick={() => window.location.reload()}>
          Reload
        </button>
      </div>
    );
  }
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </React.StrictMode>,
);
