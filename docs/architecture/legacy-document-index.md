# Índice de documentação histórica e hosts aposentados

**Revisão:** 2026-11-16.
**Host de produto ativo:** Slint 1.18.x + WGPU 30.x/Vulkan + CPAL 0.18.2 + SDL3/input runtime.

Este índice evita que documentos históricos sejam confundidos com contratos de implementação atuais. A presença de código ou documentação antiga não significa que o host correspondente esteja ativo.

## Hosts e interfaces anteriores

| Documento | Classificação | Uso permitido |
|---|---|---|
| `architecture/egui-frontend-debug-ui.md` | HISTÓRICO / SUPERSEDED | Referência de organização de menus e diagnósticos |
| `architecture/native-gui-d3d12-egui-composition.md` | HISTÓRICO / SUPERSEDED | Histórico de composição do host D3D12 |
| `architecture/qt-frontend-functional-model.md` | HISTÓRICO / SUPERSEDED | Requisitos conceituais somente após revalidação |
| `architecture/qt-frontend-target-architecture.md` | HISTÓRICO / SUPERSEDED | Não usar como arquitetura-alvo |
| `architecture/qt-sms-input-settings.md` | HISTÓRICO / SUPERSEDED | Referência de UI antiga |
| `architecture/vulkan-presentation-qt-quick.md` | HISTÓRICO / SUPERSEDED | Protótipo Qt Quick/Vulkan, não implementar |
| `architecture/clean-architecture-migration.md` | HISTÓRICO / SUPERSEDED | Migração Qt/PySide6 antiga |
| `architecture/host-driver-apis.md` | HISTÓRICO / SUPERSEDED | Contratos Qt/XInput/WASAPI antigos |
| `architecture/audio-output-backends.md` | HISTÓRICO / SUPERSEDED | WASAPI/XAudio2 antigo; CPAL é o caminho atual |
| `architecture/native-audio-runtime.md` | HISTÓRICO / SUPERSEDED | Runtime anterior de áudio; preservar apenas evidências úteis |
| `sg1000-first-playable.md` | MARCO HISTÓRICO DE HOST | Referência de comportamento/core SG-1000; comandos Qt/Python não ativos |
| `validation/qt-frontend-pr98.md` | RELATÓRIO HISTÓRICO | Não comprova validação Slint/WGPU atual |

## ADRs históricas

ADRs de egui/eframe, D3D12 e Qt devem permanecer para rastreabilidade de decisões anteriores. Sua autoridade depende do estado/status explícito e da precedência registrada em [ADRs](../adr/README.md), [arquitetura atual](../architecture.md) e [ADR 0011](../adr/0011-slint-wgpu-cpal-desktop-platform.md).

## Regras de preservação

- Não apagar histórico técnico, resultados ou decisões antigas.
- Não implementar funcionalidades novas em hosts aposentados.
- Não tratar validação de Qt/egui/D3D12 como validação do host Slint/WGPU.
- Antes de remover módulos legados, mapear consumidores, features, workspaces, testes e workflows.
- Portar seletivamente regras de hardware ainda válidas; nunca copiar um runtime antigo inteiro para criar uma segunda implementação.
