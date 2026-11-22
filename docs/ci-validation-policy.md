# Política de CI e validação

**Baseline de auditoria:** `main` em `82ccad8eb9cd11d78dfddd168f307f90012426b5` (2026-11-11).
**Escopo:** runtime Rust, hardware, hosts transitórios e migração Slint/WGPU/CPAL/SDL3.

Esta política é subordinada aos ADRs vigentes e ao código integrado na `main`. PRs Draft, branches e validações parciais não alteram o estado integrado.

## 1. Princípios

1. A CI seleciona a menor suíte que cubra o código alterado e seus consumidores reais.
2. A seleção por caminhos nunca pode ocultar uma regressão: dependências compartilhadas acionam a união dos gates dos consumidores.
3. Testes de hardware/runtime permanecem obrigatórios nas alterações pertinentes.
4. Hosts aposentados não recebem novos gates de build; ADRs e auditorias históricas são preservadas. Validação física é registrada separadamente.
5. Um job `SKIPPED` nunca é `PASS`. O gate agregado exige `success` para jobs aplicáveis e `skipped` para jobs não aplicáveis.
6. Cada resultado deve ser associado ao SHA exato avaliado.
7. Builds de aplicação são Release e usam lockfiles versionados com `--locked`.
8. Execução física em GPU, dispositivos de áudio ou gamepads é registrada separadamente; compilação e testes headless não comprovam operação física.

## 2. Workflows ativos

| Workflow | Responsabilidade |
|---|---|
| `.github/workflows/ci.yml` | Classificação de impacto, regressão Rust, gates focados por domínio, hosts condicionais, desktop Slint/WGPU, higiene documental e lint de workflows |
| `.github/workflows/sms-debugger-runtime.yml` | Integração SMS runtime Windows x64 Release e benchmark YM2413; não compila GUI separada |
| `.github/workflows/ym2413-pg-differential.yml` | Diferencial serial PG/operadores contra Nuked, somente para fontes e ferramentas PG relacionadas |
| `.github/workflows/ym2413-eg-characterization.yml` | Sweep e replay EG; somente para fontes, fixtures e ferramentas de envelope |
| `.github/workflows/ym2413-rhythm-reset-differential.yml` | Caracterização adicional de reset/ruído Rhythm Mode; execução manual sob demanda |
| `.github/workflows/ym2413-output-pipeline-differential.yml` | Caracterização isolada adicional do pipeline MO/RO; execução manual sob demanda |

As alterações em `.github/workflows/**`, `.github/actions/**` ou `.github/scripts/**` acionam o gate `workflow-lint` e a regressão do workspace Rust. A análise estática dos workflows utiliza actionlint v1.7.12, instalado por release versionada e SHA-256 verificado.

## 3. Matriz de rastreabilidade

| Componente | Validação obrigatória | Workflow | Condição |
|---|---|---|---|
| YM2413 | fmt, Clippy Release e testes do crate; integração no SMS runtime Windows Release | CI / YM2413 focused + SMS Debugger Runtime | Alterações em `native/ym2413/**`; substitui a regressão genérica do workspace quando a alteração está isolada no YM2413 |
| YM2413 PG/EG/Rhythm/MO-RO | Diferenciais com Nuked-OPLL, restritos aos arquivos e fixtures de cada domínio | Workflows YM2413 especializados | Executar apenas quando os componentes/fontes/fixtures correspondentes mudarem; execução manual disponível para auditoria ampliada |
| Runtime Rust | fmt, Clippy Release, testes unitários e integração | CI / Rust workspace | Crates Rust gerais, manifests, lockfiles e configuração global |
| Z80 CPU/ALU/bus | fmt, Clippy, testes semânticos e integração SMS/SG-1000 | CI / Z80 focused | Alterações nos módulos Z80 e integrações classificadas |
| Z80 vetores externos | SingleStepTests DD/FD indexados, revisão de estado arquitetural e T-states | CI / Z80 focused | Alterações Z80 |
| PSG | fmt, Clippy e testes do PSG e consoles | CI / PSG focused | Alterações PSG |
| VDP | fmt, Clippy e testes do VDP e consoles | CI / VDP focused | Alterações VDP |
| MachineScheduler/temporal core | Regressão do workspace e replay determinístico | CI / Scheduler stress | Alterações temporais ou opt-in por label |
| Input/SDL3 | fmt, Clippy, testes do runtime de entrada e validação Windows do backend SDL3 | CI / Input focused + Windows SDL3 | Alterações em input runtime, integração SMS/SG-1000 ou host Slint |
| Slint/WGPU 30 | fmt, Clippy, testes e build Windows x64 Release | CI / Slint WGPU | Alterações em `native/sega_desktop` ou dependências do workspace |
| Vulkan físico | Janela, device/queue, textura externa, composição, resize e teardown em GPU real | Validação manual R1 | Antes de declarar R1 concluída |
| CPAL output | fmt, Clippy, testes do backend e build Windows x64 | CI / CPAL | Alterações em `native/audio_output/**`, áudio runtime e dependências relacionadas |
| Documentação | `git diff --check` | CI / Documentation | Alterações Markdown, MDX, RST ou TXT |
| Workflows | actionlint e validação do gate agregado | CI / Workflow lint | Alterações nos workflows e arquivos auxiliares |
| PowerShell tooling | Parser PowerShell nos scripts alterados e auditoria de workspaces/lockfiles | CI / Windows tooling | Alterações em `scripts/**/*.ps1` ou `atualizar_recompilar_sega_emulator.ps1` |
| Lockfiles | `cargo metadata --locked` e existência dos lockfiles | CI / Rust workspace e lockfile artifact | Alterações de manifests, lockfiles ou refresh manual |

## 4. Regras de impacto e dependências

- Z80-only executa a suíte focada Z80, incluindo os testes integrados SMS/SG-1000 e vetores externos pertinentes. Não exige compilar hosts gráficos ou de áudio sem relação com a alteração.
- YM2413-only executa o gate focado do crate e a integração do SMS runtime Windows Release; não executa a regressão genérica do workspace nem o job CPAL.
- O workflow EG é limitado aos módulos de envelope, core/slot sequencer, exemplos, fixtures e ferramentas EG. Alterações exclusivas em PG/operadores/MO-RO não disparam o sweep de envelopes.
- O diferencial PG automático já inclui a comparação integrada de Rhythm Mode e o pipeline MO/RO. As caracterizações isoladas de reset/ruído e MO/RO permanecem disponíveis por `workflow_dispatch`, mas não consomem runners em toda alteração do serviço serial.

- PSG, VDP e input têm jobs de pacote com integração de console. Eles substituem a regressão genérica do workspace somente quando a alteração está isolada nesses domínios.
- MachineScheduler e temporal core executam a regressão do workspace e o stress replay determinístico. Consumidores legados afetados também são validados quando necessário.
- Alterações em `native/observability` acionam os consumidores pertinentes.
- Alterações em `rust/runtime/sms-runtime` acionam a regressão Rust e o workflow SMS Runtime Release.
- Alterações em `native/audio_output` executam os gates do backend CPAL; testes headless não substituem a validação física de dispositivos.
- Alterações de workflow executam actionlint e a regressão do workspace, mas não forçam compilações de hosts legados sem relação com a alteração.
- Alterações de documentação executam higiene de diff, sem build Rust indiscriminado.

Os filtros que combinam inclusões e exclusões usam `predicate-quantifier: some-with-excludes`. O gate agregado compara explicitamente o resultado real de cada job com sua aplicabilidade esperada.

## 5. Componentes aposentados e validações físicas pendentes

### egui/eframe, D3D12 e WASAPI/XAudio2

Esses hosts foram retirados do caminho ativo. A GUI independente `native/sms_debugger`, seus manifests e suas dependências exclusivas `eframe`/`rfd` foram removidos. O núcleo `sega-debugger-core` permanece no workspace e no SMS runtime. ADRs e relatórios datados são mantidos somente como histórico; não são alvos de build nem autorizam reintroduzir essas tecnologias.

### CPAL

CPAL 0.18.2 é o backend ativo de saída física e está integrado ao host Slint. A CI cobre compilação e testes de software. Enumeração real de endpoints, formatos negociados, reprodução prolongada, hotplug/reconexão e underruns em dispositivos físicos exigem validação Windows separada.

### SDL3

SDL3 está integrado ao host Slint para gamepads. Os testes do runtime e a validação Windows cobrem o caminho de software; hotplug e comportamento com controles físicos devem ser registrados separadamente quando validados.

### Scripts de desenvolvimento

- `scripts/project.ps1` audita o workspace raiz e o workspace independente CPAL, incluindo manifests e lockfiles.
- `scripts/compile-fast.ps1` verifica o workspace raiz; use `-AllWorkspaces` para incluir o workspace CPAL independente.
- `scripts/install_sdl3_runtime.ps1` prepara manualmente a DLL SDL3 oficial com checksum fixado; não substitui os gates de integração.
- `scripts/common.ps1` fornece a inicialização MSVC compartilhada pelos scripts Windows.

A CI analisa sintaticamente scripts PowerShell alterados e executa `project.ps1 audit` e `project.ps1 locks` no gate de tooling.

## 6. Testes que devem permanecer protegidos

Inventário de arquivos de integração encontrado na baseline:

| Domínio | Arquivos |
|---|---:|
| Z80 CPU | 30 |
| Z80 integrado ao SMS | 48 |
| SMS temporal/estado/interrupções | 4 |
| MachineScheduler | 3 |
| VDP | 1 |
| SMS runtime API | 1 |
| **Total** | **87** |

Esse inventário conta arquivos de integração versionados; não inclui testes unitários internos aos módulos Rust.

O teste `native/machine_scheduler/tests/stress_replay.rs` é executado como teste ignorado por um job dedicado, com limite de memória. Não remover o gate nem classificar a ausência de execução como aprovação.

A validação externa DD/FD existente é um conjunto de vetores SingleStepTests específico. Ela não é ZEXDOC/ZEXALL e não altera a regra de que E8 só começa após o fechamento demonstrável de E7.

Não foram encontrados arquivos de testes Python na árvore desta baseline. Caso ferramentas Python voltem a fazer parte do backend, deverão receber gates próprios, sem substituir os testes Rust.

## 7. Lockfiles e workspaces

Workspaces com lockfile independente devem ser auditados individualmente:

- `Cargo.lock` — workspace Rust raiz, incluindo `native/sega_desktop`.
- `native/audio_output/Cargo.lock` — workspace independente do backend CPAL.

Não existem workspaces ativos para `native/sega_app` ou `native/sms_debugger`; seus manifests e dependências de GUI foram retirados. `native/debugger_core` permanece no workspace raiz porque é consumido pelo SMS runtime.

O refresh de lockfiles é uma operação manual/opt-in que produz artefato temporário para revisão. O workflow não recebe permissão `contents: write`, não faz commit e não executa push automático.

`scripts/project.ps1` audita o workspace raiz e o workspace CPAL independente explicitamente. Manifests fora dos workspaces reconhecidos são classificados como `NOT RUN`, nunca como validados.

## 8. Segurança e permissões

- Permissões padrão mínimas; jobs de validação usam `contents: read`.
- Checkout com `persist-credentials: false`.
- Actions externas referenciadas por SHA imutável e com versão anotada.
- Downloads de ferramentas são versionados e verificados por checksum.
- Nenhum workflow de CI de rotina recebe permissão de escrita para atualizar branches.
- Alterações de workflows devem ser revisadas como código executável, incluindo filtros, expressões, permissões e scripts inline.

## 9. Classificação de resultados

| Estado | Significado |
|---|---|
| PASS | Gate aplicável executado e concluído com sucesso no SHA registrado |
| FAIL | Gate aplicável executado e falhou |
| BLOCKED | Gate não pôde ser executado por pré-requisito ausente |
| NOT RUN | Validação não executada; sem evidência de aprovação |
| SKIPPED | Job não executado por condição de impacto; não equivale a PASS |

A PR só pode ser integrada quando todos os gates aplicáveis ao diff estiverem em `success`, os não aplicáveis estiverem realmente `skipped`, e não houver bloqueios técnicos pendentes. A autorização de merge não substitui os critérios de aceitação.
