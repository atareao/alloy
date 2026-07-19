import { Button, Group, Modal, Paper, Stack, Text } from "@mantine/core";
import type { BatchResults } from "./BatchProgress";

export interface SummaryDialogProps {
  opened: boolean;
  onClose: () => void;
  checkResults: BatchResults;
  updateResults: BatchResults;
  phase: string;
}

export default function SummaryDialog({
  opened,
  onClose,
  checkResults,
  updateResults,
  phase,
}: SummaryDialogProps) {
  if (phase !== "idle") return null;

  const hasUpdates =
    updateResults.done > 0 || updateResults.failed > 0;
  const allErrors = [...checkResults.errors, ...updateResults.errors];

  return (
    <Modal
      opened={opened}
      onClose={onClose}
      title={
        hasUpdates
          ? "📋 Resumen de actualización"
          : "📋 Resumen de comprobación"
      }
      size="sm"
    >
      <Stack gap="md">
        {/* Check results */}
        <Paper p="md" withBorder>
          <Text size="sm" c="dimmed" mb="xs">
            🔍 Comprobación
          </Text>
          <Group gap="lg">
            <Stack gap="0" align="center">
              <Text size="xl" fw={700}>
                {checkResults.total}
              </Text>
              <Text size="xs" c="dimmed">
                Comprobados
              </Text>
            </Stack>
            <Stack gap="0" align="center">
              <Text size="xl" fw={700} c="yellow">
                {checkResults.updated}
              </Text>
              <Text size="xs" c="dimmed">
                Actualizables
              </Text>
            </Stack>
            <Stack gap="0" align="center">
              <Text size="xl" fw={700} c="green">
                {checkResults.uptodate}
              </Text>
              <Text size="xs" c="dimmed">
                Actuales
              </Text>
            </Stack>
            <Stack gap="0" align="center">
              <Text size="xl" fw={700} c="red">
                {checkResults.failed}
              </Text>
              <Text size="xs" c="dimmed">
                Errores
              </Text>
            </Stack>
          </Group>
        </Paper>

        {/* Update results */}
        {hasUpdates && (
          <Paper p="md" withBorder>
            <Text size="sm" c="dimmed" mb="xs">
              ⬆️ Actualización
            </Text>
            <Group gap="lg">
              <Stack gap="0" align="center">
                <Text size="xl" fw={700} c="green">
                  {updateResults.done}
                </Text>
                <Text size="xs" c="dimmed">
                  Actualizados
                </Text>
              </Stack>
              <Stack gap="0" align="center">
                <Text size="xl" fw={700} c="red">
                  {updateResults.failed}
                </Text>
                <Text size="xs" c="dimmed">
                  Fallos
                </Text>
              </Stack>
            </Group>
          </Paper>
        )}

        {/* Errors */}
        {allErrors.length > 0 && (
          <Paper p="sm" withBorder bg="red.0">
            <Text size="xs" fw={500} mb="xs" c="red">
              Errores:
            </Text>
            {allErrors.map((err, i) => (
              <Text key={i} size="xs" c="red">
                {err}
              </Text>
            ))}
          </Paper>
        )}

        <Group justify="flex-end">
          <Button onClick={onClose}>Cerrar</Button>
        </Group>
      </Stack>
    </Modal>
  );
}
