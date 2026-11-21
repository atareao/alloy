import { Card, Flex, Switch, Typography } from "antd";

interface ThemeCardProps {
  colorScheme: "dark" | "light";
  setColorScheme: (scheme: "dark" | "light") => void;
}

/** Card «🌙/☀️ Tema» (pestaña Información). */
export default function ThemeCard({ colorScheme, setColorScheme }: ThemeCardProps) {
  return (
    <Card bordered size="small">
      <Flex justify="space-between" align="center">
        <div>
          <Typography.Title level={4} style={{ margin: 0 }}>
            {colorScheme === "dark" ? "🌙" : "☀️"} Tema
          </Typography.Title>
          <Typography.Text type="secondary">
            {colorScheme === "dark" ? "Modo oscuro" : "Modo claro"}
          </Typography.Text>
        </div>
        <Switch
          checked={colorScheme === "dark"}
          onChange={(checked) => {
            const next = checked ? "dark" : "light";
            localStorage.setItem("color-scheme", next);
            setColorScheme(next);
          }}
          checkedChildren="🌙"
          unCheckedChildren="☀️"
        />
      </Flex>
    </Card>
  );
}
