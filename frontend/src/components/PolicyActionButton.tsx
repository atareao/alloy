import { useState } from "react";
import { Button, Group, Modal, Select, Stack, Switch, Text } from "@mantine/core";
import { apiFetch } from "../api";
import type { UpdatePolicy, UpdateAction } from "../types";

interface PolicyActionButtonProps {
  containerName: string;
  isSingleUpdating: boolean;
  updateSingleContainer: (name: string) => Promise<void>;
  getPolicy: (name: string) => UpdatePolicy | undefined;
  setPolicies: React.Dispatch<React.SetStateAction<UpdatePolicy[]>>;
  btnSize: string;
  busy: boolean;
  hasUpdate: boolean;
  showToast: (message: string, color: string) => void;
}

export default function PolicyActionButton({
  containerName, isSingleUpdating, updateSingleContainer, getPolicy, setPolicies, btnSize, busy, hasUpdate, showToast,
}: PolicyActionButtonProps) {
  const [showPolicyModal, setShowPolicyModal] = useState(false);
  const [editAction, setEditAction] = useState<string>('pull-restart');
  const [editCleanup, setEditCleanup] = useState(false);
  const [editRollback, setEditRollback] = useState(false);
  const [savingPolicy, setSavingPolicy] = useState(false);

  const policy = getPolicy(containerName);
  const action = policy?.action || 'pull-restart';

  const openConfig = () => {
    setEditAction(policy?.action || 'pull-restart');
    setEditCleanup(policy?.cleanup_old_image || false);
    setEditRollback(policy?.rollback_on_failure || false);
    setShowPolicyModal(true);
  };

  const savePolicy = async () => {
    setSavingPolicy(true);
    try {
      const res = await apiFetch(`/api/update-policies/${encodeURIComponent(containerName)}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: editAction,
          cleanup_old_image: editCleanup,
          rollback_on_failure: editRollback,
        }),
      });
      if (res.ok) {
        const updated: UpdatePolicy = await res.json();
        setPolicies(prev => {
          const next = prev.filter(p => p.container !== containerName);
          next.push(updated);
          return next;
        });
        setShowPolicyModal(false);
        showToast(`⚙️ Política de ${containerName} actualizada ✅`, "green");
      }
    } catch {
      showToast(`⚙️ Error al guardar política`, "red");
    }
    setSavingPolicy(false);
  };

  const labelMap: Record<string, string> = {
    'none': '⬆ Pendiente',
    'pull': '⬇️ Pull',
    'pull-restart': '⬆ Actualizar',
    'pull-restart-stack': '📦 Actualizar stack',
  };
  const colorMap: Record<string, string> = {
    'none': 'gray',
    'pull': 'cyan',
    'pull-restart': 'yellow',
    'pull-restart-stack': 'violet',
  };

  const handleAction = () => {
    if (action === 'none' || !hasUpdate) {
      openConfig();
      return;
    }
    updateSingleContainer(containerName);
  };

  return (
    <>
      <Button
        size={btnSize}
        variant="filled"
        color={colorMap[action] || 'yellow'}
        leftSection={hasUpdate ? "⬆" : "⚙️"}
        onClick={handleAction}
        loading={isSingleUpdating}
        disabled={busy}
      >
        {hasUpdate ? (labelMap[action] || 'Actualizar') : 'Configurar'}
      </Button>
      <Modal opened={showPolicyModal} onClose={() => setShowPolicyModal(false)} title={`⚙️ Política: ${containerName}`} size="md">
        <Stack>
          <Text size="sm" c="dimmed" mb="xs">
            Configura qué hacer cuando haya una actualización disponible para este contenedor.
          </Text>
          <Select
            label="Acción"
            data={[
              { value: 'none', label: '❌ No hacer nada' },
              { value: 'pull', label: '⬇️ Pull imagen' },
              { value: 'pull-restart', label: '🔄 Pull + reiniciar contenedor' },
              { value: 'pull-restart-stack', label: '📦 Pull + reiniciar stack' },
            ]}
            value={editAction}
            onChange={(v) => v && setEditAction(v)}
          />
          <Switch
            label="🧹 Borrar imagen anterior"
            description="Elimina la imagen anterior después de actualizar"
            checked={editCleanup}
            onChange={(e) => setEditCleanup(e.currentTarget.checked)}
          />
          <Switch
            label="↩️ Rollback si falla"
            description="Si el contenedor no arranca correctamente, restaura la imagen anterior"
            checked={editRollback}
            onChange={(e) => setEditRollback(e.currentTarget.checked)}
          />
          <Group justify="flex-end" mt="md">
            <Button variant="default" onClick={() => setShowPolicyModal(false)}>Cancelar</Button>
            <Button onClick={savePolicy} loading={savingPolicy}>Guardar política</Button>
          </Group>
        </Stack>
      </Modal>
    </>
  );
}