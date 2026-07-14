import { Component, type ErrorInfo, type ReactNode } from "react";
import { Container, Paper, Text, Title, Button, Stack, Code } from "@mantine/core";

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
}

export default class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null, errorInfo: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error, errorInfo: null };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("ErrorBoundary caught:", error, errorInfo);
    this.setState({ errorInfo });
  }

  render() {
    if (this.state.hasError) {
      return (
        <Container py="xl">
          <Paper shadow="sm" p="lg" withBorder>
            <Title order={3} mb="md" c="red">
              ⚠️ Algo salió mal
            </Title>
            <Stack gap="xs" mb="md">
              <Text size="sm">
                {this.state.error?.message || "Error desconocido"}
              </Text>
              {this.state.error?.stack && (
                <Code block style={{ whiteSpace: "pre-wrap", fontSize: "0.75rem", maxHeight: 300, overflow: "auto" }}>
                  {this.state.error.stack}
                </Code>
              )}
              {this.state.errorInfo?.componentStack && (
                <Code block style={{ whiteSpace: "pre-wrap", fontSize: "0.7rem", color: "gray" }}>
                  {this.state.errorInfo.componentStack}
                </Code>
              )}
            </Stack>
            <Button
              variant="light"
              onClick={() => {
                this.setState({ hasError: false, error: null, errorInfo: null });
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
