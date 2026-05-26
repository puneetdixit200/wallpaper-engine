import { Component, ErrorInfo, ReactNode } from "react";

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
  message: string;
}

export class ErrorBoundary extends Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  state: ErrorBoundaryState = {
    hasError: false,
    message: "",
  };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return {
      hasError: true,
      message: error.message,
    };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("Wallpaper Engine render error", error, info.componentStack);
  }

  render() {
    if (this.state.hasError) {
      return (
        <main className="app-shell error-shell">
          <section className="error-panel" role="alert">
            <p className="eyebrow">Render error</p>
            <h1>Wallpaper Engine needs a reload</h1>
            <p>{this.state.message || "An unexpected UI error occurred."}</p>
          </section>
        </main>
      );
    }

    return this.props.children;
  }
}
