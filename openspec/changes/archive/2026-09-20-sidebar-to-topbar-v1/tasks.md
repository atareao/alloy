# Tasks: sidebar-to-topbar-v1

## 1. Eliminar Layout.Sider y estado asociado

- [x] 1.1 Eliminar import de `Menu` de antd
- [x] 1.2 Eliminar import de `MenuUnfoldOutlined` de @ant-design/icons
- [x] 1.3 Eliminar estado `siderCollapsed` (línea 437)
- [x] 1.4 Eliminar `Layout.Sider` completo (líneas 444-480)
- [x] 1.5 Eliminar botón flotante de hamburguesa (líneas 566-578)

## 2. Transformar Layout.Header en topbar con navegación

- [x] 2.1 Añadir import de `Space` de antd
- [x] 2.2 En `Layout.Header`, añadir botones de navegación a la derecha con Flex justify="space-between"
- [x] 2.3 Botones: Dashboard (BarChartOutlined), Historial (FileTextOutlined), Config (SettingOutlined), Salir (LogoutOutlined)
- [x] 2.4 En desktop: botones con icono + texto
- [x] 2.5 En mobile: botones con solo icono
- [x] 2.6 Botón activo destacado visualmente (type="primary")

## 3. Verificación

- [x] 3.1 Ejecutar `npx vitest run` para tests frontend — 26 passed ✅
- [x] 3.2 Ejecutar `cargo test` para tests backend — 181 passed (4 integration tests require Docker) ✅
- [x] 3.3 Ejecutar `cargo clippy -- -D warnings` — clean ✅
- [x] 3.4 Ejecutar `cargo fmt --check` — clean ✅
- [x] 3.5 Ejecutar `npx tsc --noEmit` — clean ✅