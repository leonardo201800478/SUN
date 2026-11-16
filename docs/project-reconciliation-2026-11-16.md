# Reconciliação geral do projeto — 2026-11-16

**Baseline remota:** `main` SHA `bb155bdfe1a188ccc8f5b4a01896b23a0cfde808`.
**Escopo:** estado, PRs, correções de autoridade documental e nova fase de expansões/acessórios. Somente documentação.

## 1. Estado remoto

- Árvore: 285 arquivos Markdown em `docs/`.
- CI baseline: runs #43945796678 e #43945796477. Quatro checks SUCCESS e 14 SKIPPED. PASS: Rust quality/unit, Z80 focused, change impact e SMS Runtime Release format/Clippy/tests. SKIPPED não valida o subsistema.
- PRs abertas observadas: #978, #974, #533, #921, #878, #863, #769, #881, #913 e #928.

## 2. Correções de estado fornecidas pelo usuário

| Item | Estado atual a registrar | Limite |
|---|---|---|
| Host gráfico anterior | Fora de uso ativo; código remanescente é legado | Inventariar consumidores antes de remover |
| Road Rash | Resolvido e testado fisicamente | Relato do usuário; não inventar SHA/log |
| Alinhamento de sprites SMS | Resolvido | Relato do usuário; reabrir somente por regressão |
| BIOS boot | Validação física pendente | Não declarar aceite físico |
| Debugger | Implementado parcialmente; precisa evoluir | Trilha independente |
| Z80 E7 | `CLOSED_WITH_LIMITATIONS` no escopo praticável | Não alegar equivalência elétrica universal |
| SG-1000 | Próxima prioridade de hardware | Concluir na arquitetura temporal existente, sem duplicação |

## 3. Z80 — encerramento com limitações

A main contém as integrações E7 mais recentes (#955–#969), incluindo flags de operações de bloco, WZ/MEMPTR, exceções DD/FD, requests e integração temporal dirigida. A baseline executou PASS no gate Z80 focused.

E7 fica `CLOSED_WITH_LIMITATIONS` para o perfil Zilog NMOS delimitado e as necessidades praticáveis de SMS/SG-1000. Revisão independente por outra IA, console físico ou ZEXDOC/ZEXALL não são bloqueios permanentes.

Limitações: CMOS/clones não caracterizados; WAIT em fixture não prova WAIT físico; DMA e sincronização física global não comprovados; ROM v5.0/Woodmass têm escopo específico; ZEXDOC/ZEXALL não executados. Não declarar TEMPORAL_PASS sistêmico. Não reabrir sem bug/regressão reproduzível.

## 4. SMS e debugger

Road Rash e alinhamento de sprites saem do backlog de defeitos conforme relato do usuário. BIOS boot continua com validação física pendente; PR #928 é candidata de implementação, não aceite físico. A PR #921 registra RunTest32/34 como FAIL na sua própria execução; não atribuir correção/estado ao HEAD atual sem nova execução. O debugger permanece parcial; PR #878 não cobre ainda toda a solução de diagnóstico.

## 5. SG-1000

Concluir a máquina temporal conectando os componentes existentes: Z80 compartilhado, MachineScheduler, memória/I/O, TMS9918A e SN76489. Fechar coordenação CPU/VDP/PSG, clocks, side effects, WAIT quando demonstrado, snapshots compostos, restore/replay e equivalência contínua/em blocos. Não criar CPU, GUI, pipeline ou Scheduler paralelo.

## 6. Roadmap

SMS1/SMS2 → SG-1000 → Game Gear → expansões/acessórios 8-bit → Mega Drive/Genesis → Sega CD → 32X.

A fase de acessórios inclui YM2413/FM Sound Unit, Sega Card/My Card, Card Catcher para SG-1000/SC-3000, 3D Glasses, Paddle, Sports Pad, Light Phaser, Rapid Fire e interfaces de expansão. Ver `hardware/sms-expansion-accessories-roadmap.md`.

## 7. PRs abertas observadas

| PR | Leitura atual |
|---|---|
| #978 | E7 documental baseada em main anterior e bloqueada por revisão independente; superseder pela nova política |
| #974 | SG-1000 adiado até E7 formalmente fechado; atualizar dependência |
| #533 | Contrato final Draft; atualizar após estabilização do roadmap |
| #921 | VCounter HPos23; RunTest32/34 FAIL na evidência da PR; manter até nova validação |
| #878 | Debugger parcial; manter trilha independente |
| #863 | Scheduler optimization Draft sem validação local declarada; não priorizar sem benefício demonstrado |
| #769 | Isolamento D3D12; não é requisito do host ativo |
| #881 | Investigação de sprites; alinhamento reportado resolvido |
| #913 | Road Rash; resolvido por teste físico reportado |
| #928 | BIOS; implementação candidata, validação física pendente |

A tabela não declara merge nem fechamento. Ações no GitHub exigem estado remoto revalidado e comentário adequado.

## 8. Inventário documental

A árvore contém 285 Markdown; contagem estrutural não equivale a leitura integral. Próxima etapa: agrupar por autoridade/sistema/componente/data; classificar vigente/histórico/substituído/obsoleto; reconciliar índices e links; mapear documento → módulo/símbolo → teste → workflow → SHA. Preservar histórico e não apagar evidência por idade.


## 9. Estado das PRs após a reconciliação — 2026-11-16

- **#979 — aberta em Draft:** consolidação documental, portabilidade das evidências E7 e novo escopo de acessórios. Base `bb155bdfe1a188ccc8f5b4a01896b23a0cfde808`; HEAD documental atual a consultar no GitHub. Sem alteração de produção.
- **#978 — fechada sem merge:** conteúdo documental E7 portado seletivamente para #979.
- **#974 — fechada sem merge:** plano SG-1000 P2 portado e atualizado em #979.
- **#769 — fechada sem merge:** isolamento do D3D12 supersedido; host anterior fora de uso ativo. Código legado não foi removido.
- **#881 — fechada sem merge:** desalinhamento de sprites reportado pelo usuário como corrigido. Investigações VDP independentes permanecem separadas.
- **#913 — fechada sem merge:** Road Rash reportado pelo usuário como testado fisicamente e OK.
- **#533 — permanece Draft:** contrato de conclusão final; requer reconciliação posterior com o roadmap novo.
- **#921 — permanece Draft:** correção VCounter; sua própria evidência full-ROM ainda registra falhas RunTest32/34. Não integrar sem nova validação.
- **#878 — permanece Draft:** evolução do debugger; captura/exportação/snapshots ainda incompletos.
- **#863 — permanece Draft:** otimização Scheduler; aguarda validação aplicável e não precede SG-1000 por padrão.
- **#928 — permanece Draft:** boot por BIOS convencional; validação física pendente.

Os fechamentos foram acompanhados de comentários explícitos. Nenhuma dessas PRs foi integrada por esta reconciliação.


## 10. Substituição da PR documental — 2026-11-16

A PR #979 foi fechada sem merge para evitar um histórico de 67 commits gerados por operações arquivo a arquivo. Seu Git tree documental foi preservado integralmente e reapresentado na PR #980, em branch limpa baseada diretamente na main `bb155bdfe1a188ccc8f5b4a01896b23a0cfde808`. A PR #980 é a candidata ativa; a #979 permanece fechada/supersedida. A PR #980 aguarda CI no seu próprio HEAD. Nenhuma PR foi integrada à main por esta reconciliação.
