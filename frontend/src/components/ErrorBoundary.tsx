import { Component, type ErrorInfo, type ReactNode } from "react";
import { Card, Typography, Button, Space } from "antd";
import { WarningOutlined, ReloadOutlined } from "@ant-design/icons";

const { Text, Title } = Typography;

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
}

export default class ErrorBoundary extends Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
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
        <div
          style={{
            display: "flex",
            justifyContent: "center",
            padding: "var(--ant-padding-xl)",
          }}
        >
          <Card variant="outlined" style={{ maxWidth: 600, width: "100%" }}>
            <Title level={3} style={{ color: "var(--ant-color-error)" }}>              
              <WarningOutlined style={{ marginRight: 8 }} />
              Algo salió mal
            </Title>
            <Space direction="vertical" size="small" style={{ width: "100%", marginBottom: "var(--ant-margin-md)" }}>
              <Text>{this.state.error?.message || "Error desconocido"}</Text>
              {this.state.error?.stack && (
                <pre
                  style={{
                    whiteSpace: "pre-wrap",
                    fontSize: "0.75rem",
                    maxHeight: 300,
                    overflow: "auto",
                    background: "var(--ant-color-bg-container)",
                    border: "1px solid var(--ant-color-border)",
                    borderRadius: "var(--ant-border-radius)",
                    padding: "var(--ant-padding-xs)",
                    margin: 0,
                    color: "var(--ant-color-text)",
                  }}
                >
                  {this.state.error.stack}
                </pre>
              )}
              {this.state.errorInfo?.componentStack && (
                <pre
                  style={{
                    whiteSpace: "pre-wrap",
                    fontSize: "0.7rem",
                    color: "var(--ant-color-text-secondary)",
                    background: "var(--ant-color-bg-container)",
                    border: "1px solid var(--ant-color-border)",
                    borderRadius: "var(--ant-border-radius)",
                    padding: "var(--ant-padding-xs)",
                    margin: 0,
                  }}
                >
                  {this.state.errorInfo.componentStack}
                </pre>
              )}
            </Space>
            <Button
              type="default"
              icon={<ReloadOutlined />}
              onClick={() => {
                this.setState({
                  hasError: false,
                  error: null,
                  errorInfo: null,
                });
                window.location.reload();
              }}
            >
              Recargar página
            </Button>
          </Card>
        </div>
      );
    }
    return this.props.children;
  }
}