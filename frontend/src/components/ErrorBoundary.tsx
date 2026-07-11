import { Component, type ErrorInfo, type ReactNode } from "react";
import { Container, Paper, Text, Title, Button } from "@mantine/core";

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

export default class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, _errorInfo: ErrorInfo) {
    console.error("ErrorBoundary caught:", error);
  }

  render() {
    if (this.state.hasError) {
      return (
        <Container py="xl">
          <Paper shadow="sm" p="lg" withBorder>
            <Title order={3} mb="md" c="red">
              ⚠️ Algo salió mal
            </Title>
            <Text size="sm" mb="md">
              {this.state.error?.message || "Error desconocido"}
            </Text>
            <Button
              variant="light"
              onClick={() => {
                this.setState({ hasError: false, error: null });
                window.location.reload();
              }}
            >
              Recargar página
            </Button>
          </Paper>
        </Container>
      );
    }
    return this.props.children;
  }
}
