import { Component, type ReactNode } from "react";
import Mascot from "./Mascot";

/** Si una sección falla, muestra un aviso en lugar de dejar la ventana en blanco */
export default class ErrorBoundary extends Component<{ children: ReactNode }, { error: Error | null }> {
  state = { error: null as Error | null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  render() {
    if (!this.state.error) return this.props.children;
    return (
      <div className="empty card">
        <Mascot mood="error" size={120} />
        <strong>Ups, esta sección tuvo un problema</strong>
        <code className="small">{this.state.error.message}</code>
        <button className="btn btn-primary" onClick={() => this.setState({ error: null })}>Reintentar</button>
      </div>
    );
  }
}
