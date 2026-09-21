import { Card, Typography, Button, Flex } from "antd";
import { KeyOutlined } from "@ant-design/icons";

const { Title, Text } = Typography;

export default function LoginScreen() {
  const handleOidcLogin = () => {
    // Redirect to OIDC provider via the backend
    window.location.href = "/api/auth/login";
  };

  return (
    <Flex justify="center" style={{ padding: "var(--ant-padding-xl)" }}>
      <Card variant="outlined" style={{ maxWidth: 400, width: "100%" }}>
        <Flex vertical align="center" gap="middle">
          <Title level={2} style={{ textAlign: "center", marginBottom: 0 }}>
            <img
              src="/icon-512x512.png"
              width="512"
              height="512"
              style={{
                verticalAlign: "middle",
                marginRight: 8,
                maxWidth: "100%",
                height: "auto",
              }}
              alt="Alloy"
            />
            Alloy
          </Title>
          <Text style={{ textAlign: "center" }}>
            Inicia sesión con tu proveedor OIDC para acceder al dashboard
          </Text>
          <Button
            onClick={handleOidcLogin}
            block
            size="large"
            icon={<KeyOutlined />}
          >
            Iniciar sesión con OIDC
          </Button>
        </Flex>
      </Card>
    </Flex>
  );
}