import { Container, Paper, Text, Title, Button } from "@mantine/core";

export default function LoginScreen() {
  const handleOidcLogin = () => {
    // Redirect to OIDC provider via the backend
    window.location.href = "/api/auth/login";
  };

  return (
    <Container size="xs" py="xl">
      <Title order={2} mb="lg" ta="center">
        <img src="/favicon.svg" width="32" height="32" style={{ verticalAlign: 'middle', marginRight: 8 }} alt="Alloy" />
        Alloy
      </Title>
      <Paper shadow="sm" p="lg" withBorder>
        <Text size="sm" mb="md" ta="center">
          Inicia sesión con tu proveedor OIDC para acceder al dashboard
        </Text>
        <Button onClick={handleOidcLogin} fullWidth size="lg">
          🔑 Iniciar sesión con OIDC
        </Button>
      </Paper>
    </Container>
  );
}