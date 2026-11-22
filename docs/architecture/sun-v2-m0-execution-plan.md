# SUN V2 — M0: Arquitetura, baseline e oráculo

- **Status:** READY FOR REVIEW.
- **Issue:** [#1253 — SUN V2 M0](https://github.com/leonardo201800478/SUN/issues/1253).
- **Documentation PR:** [#1254](https://github.com/leonardo201800478/SUN/pull/1254) (Draft; not merged).
- **Branch documental:** `docs/sun-v2-architecture-charter`.
- **Main consultada:** `30420fde1510d84834ed4e776f1a1bd1a5503b78` (2026-11-22 20:38:10 UTC).
- **Tipo:** documentação, auditoria e instrumentação; sem reescrita de hardware nesta etapa.

## 1. Estado real conhecido no início

A main já possui workspace Rust e crates em `native/*` e `rust/runtime/*`; portanto, M0 não deve criar um segundo workspace nem replicar os componentes propostos no prompt fundador.

O inventário documental consultado aponta:
- SMS1/SMS2: runtime integrado; validar somente pendências documentadas e regressões reproduzíveis.
- SG-1000: P2.2/P2.3/P2.4 integradas, mas P2 ainda sem aceite final de gameplay/host físico.
- Game Gear: scaffold/planejamento; requer auditoria independente e especificação própria.
- YM2413: ainda em desenvolvimento; não declarar FM concluído.
- Desktop: Slint/WGPU/Vulkan, CPAL e SDL3 em estados de integração distintos; validar gates no SHA atual.

A autoridade documental atual possui divergências de baseline e datas. Toda reconciliação deve preservar a proveniência e não transferir resultados entre SHAs.

## 2. Evidência de desempenho existente

A campanha PERF-01/S1 ABBA existente foi medida no commit `331fc7e97bf67964b0db6ad84ed7e27d171705d3`, não na main atual.

- Host: Ryzen 5 5600, Windows 11 Pro, Rust/Cargo 1.99.1, x86_64-pc-windows-msvc.
- ROM: OutRun (World).sms, SHA-256 `069c130135d8b61393d89f26b233509d6f9de76b0fbac53087e698a7fda0ba88`.
- Throughput cenário C (YM2413 clockado e FM mixado): Release 50.9751 FPS; ThinLTO 56.7438 FPS (+11.32%).
- O perfil `release-llvm` com ThinLTO permanece candidato opt-in; não é ainda o perfil padrão.
- O diagnóstico apontou avanço YM2413 como custo relevante, mas não isolou o custo interno do core nem demonstrou que seja o único gargalo.
- Meta anterior da issue PERF-01: >=120 frames/s uncapped/headless em OutRun com FM ativo. Essa meta passa a ser um marco intermediário histórico, não o alvo final de throughput SMS.
- Diretriz de desempenho aprovada pelo proprietário: SMS com YM2413/FM ativo deve buscar **>=400 emulated frames/s**; Megazord (Mega Drive + Sega CD + 32X integrados e sincronizados) deve buscar **>=100 emulated frames/s** quando essa plataforma estiver implementada.
- Requisito de produto separado e obrigatório: 60 FPS estáveis em tempo real, com frame time <=16.67 ms, áudio/vídeo/input integrados e validação em dispositivo modesto.
- A referência Ares exige campanha comparável com mesma ROM/cena/configuração/host e definição de métrica. O registro antigo de 395 VPS do Ares não equivale automaticamente a 395 FPS do SUN.

**Conclusão:** há evidência útil, mas ela pertence a outro SHA. Reexecutar a campanha no HEAD atual antes de tomar decisões de implementação ou promover flags de compilação.

A issue [#1233 — PERF-01](https://github.com/leonardo201800478/SUN/issues/1233) já controla a sprint de desempenho. Não duplicar sua implementação em M0.

## 3. Work packages

### M0.1 — Reconciliar a autoridade documental
- [ ] Revisar GUIA_PROJETO_SUN_V2.md contra `AGENTS.md`, `docs/project-scope.md`, `docs/project-roadmap.md`, `docs/component-status.md` e ADRs aceitas.
- [ ] Identificar conflitos de licença, JIT, Z80, scheduler, clock representation, crates e roadmap.
- [ ] Publicar matriz: decisão / documento autoritativo / conflito / proposta / aprovador.
- [ ] Não editar/remover decisões aceitas antes de uma ADR de substituição aprovada.

### M0.2 — Inventariar código e ownership
- [ ] Mapear workspace, crates, módulos, dependências e consumidores.
- [ ] Classificar cada componente KEEP / REFACTOR / ORACLE / REMOVE.
- [ ] Registrar código compartilhado e diferenças por console (SMS, SG-1000, GG).
- [ ] Identificar alocações, dispatch dinâmico, cópias e estruturas do hot path por evidência de profiler.
- [ ] Preservar implementações validadas; nenhuma reescrita durante inventário.

### M0.3 — Fixar oráculo BlastEm
- [ ] Confirmar fonte upstream canônica (Mercurial oficial) e escolher um mirror somente como transporte, se necessário.
- [ ] Registrar changeset/revision exata, licença GPL-3.0-or-later e procedimento de build.
- [ ] Definir campanhas SMS, SG-1000 e GG suportadas pelo oráculo; marcar gaps.
- [ ] Preparar geração/coleta de trace e comparação de frame/audio/bus onde disponível.
- [ ] Selecionar referências secundárias por lacuna, sem misturar versões não identificadas.
- [ ] Criar ROMs de teste próprias/redistribuíveis quando não houver conformance ROM adequada.

### M0.4 — Baseline e performance gates
- [ ] Reexecutar PERF-01 ABBA no HEAD atual com mesma ROM, configuração e host, ou declarar bloqueio se a ROM/host não estiver disponível.
- [ ] Capturar Release e perfil ThinLTO separadamente.
- [ ] Fazer profiling por função dos maiores custos do frame; distinguir medição instrumentada e throughput puro.
- [ ] Definir orçamento inicial por componente a partir dos dados, não de estimativas arbitrárias.
- [ ] Separar benchmark headless/uncapped do teste integrado com Slint/WGPU/CPAL e do aceite em dispositivo modesto.
- [ ] Preparar comparação Ares/SUN com métrica, ROM, cena, configuração e hardware equivalentes; não converter VPS em FPS por suposição.
- [ ] Preservar issue #1233 como trilha de desempenho e atualizar seus critérios para a meta SMS de 400 frames/s.
- [ ] Criar gate de regressão por benchmark estável; runners compartilhados não devem decidir regressão por wall-clock ruidoso sem baseline controlada.

### M0.5 — Contratos e agentes
- [ ] Aprovar o charter arquitetural.
- [ ] Aprovar task-spec e PR checklist.
- [ ] Definir ownership de interfaces compartilhadas.
- [ ] Garantir revisão independente de mudanças complexas; não alegar agente autônomo se não houver execução real separada.

## 4. Critérios de saída M0

M0 só fecha quando:
1. Matriz de conflitos e decisões documentais aprovada pelo proprietário.
2. Inventário de crates/módulos/consumidores completo para a família 8-bit.
3. Mapa de composição e fronteiras temporais SMS/SG-1000/GG publicado.
4. BlastEm revision/build pinned e primeiro teste diferencial reproduzível.
5. Baseline atualizada no SHA de referência, com artefatos e profiling utilizáveis.
6. Orçamento de desempenho e gates definidos para a primeira tarefa de produção.
7. Nenhuma implementação de hardware foi reescrita sem task-spec e aceite.

## 5. Primeira tarefa de produção após a auditoria mínima

A direção de execução já está definida: iniciar **F2 no SMS**, usando o Z80 compartilhado e a implementação existente como base. Antes de codificar, concluir o mapa dos pontos de invalidação, dos limites do Scheduler e dos acessos observáveis.

A implementação não deve esperar uma reescrita geral nem criar outro core. A primeira PR de produção deve ser pequena, mensurável e reversível, com o interpretador sem cache preservado para comparação diferencial.

Se F2 ficar abaixo de 400 frames/s com FM ativo, a campanha de perfil determinará se F3 é a resposta adequada. Se o gargalo dominante não for CPU decode/dispatch, corrigir o gargalo real antes de introduzir compilação dinâmica.
