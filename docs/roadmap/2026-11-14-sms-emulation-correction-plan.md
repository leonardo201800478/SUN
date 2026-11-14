# Plano de correção da emulação SMS — 2026-11-14

**Baseline registrada:** `main` = `118b96bf411248173b6215b7bf16aebdf3a66bff`  
**Perfil desktop ativo:** Master System I / Sega 315-5124 (SMS1)  
**Estado:** planejamento e gates de engenharia; correções de runtime ainda não implementadas por este documento.

Este plano consolida os relatos do usuário, o scorecard SMS1, a matriz temporal e as PRs de auditoria. Sintomas visuais são evidências de comportamento, não prova isolada da causa. Nenhuma medição física do ASIC foi realizada.

## 1. Regras de execução

1. O `MachineScheduler` permanece a única autoridade temporal.
2. Não introduzir clocks independentes, callbacks por instrução na GUI ou compensações visuais.
3. Separar semântica funcional, temporização, framebuffer lógico e apresentação WGPU.
4. Preservar SMS2, SG-1000/TMS9918A e componentes Z80 validados.
5. Cada correção exige teste discriminante que falhe antes e passe depois, além de regressões.
6. Ares/MAME são referências comparativas, não prova de silício.
7. Não solicitar novos testes físicos/manuais até haver progresso interno verificável.
8. Não iniciar Z80 E8/ZEXDOC/ZEXALL: E7 permanece IN_PROGRESS e E8 BLOCKED.
9. Compilar ou mostrar uma tela não equivale a concluir uma funcionalidade.
10. Cada PR deve ter escopo pequeno, evidência, limites conhecidos e CI no HEAD exato.

## 2. Sequência oficial

### P0 — Baseline e integridade do ambiente
**Concluído na validação local informada pelo usuário.** Checkout `main` sincronizado, sem alterações pendentes e stashes preservados. Seis processos do desktop foram observados como Responding. Isso valida somente inicialização/responsividade do host, não funcionalidade do emulador.

### P1 — Portas de dados do VDP e endereçamento VRAM
**Próxima prioridade imediata.** A campanha SMS VDP Test v1.31 registrou duas falhas exclusivas:
- `1byte wr sets rd adr`
- `1byte wr sets wr adr`

Investigar separadamente: command latch de 2 bytes, decode de endereço/código, endereços de leitura e escrita, incremento/wrap, read-ahead buffer, efeitos do data port, espelhamento e snapshot entre bytes de comando. Não assumir causa comum.

**Aceite:** dois testes discriminantes isolados, regressões de portas/VRAM/CRAM, determinismo contínuo-versus-blocos e regressões SMS2/SG-1000.

### P2 — Line IRQ, H/V counters e causalidade temporal
Investigar `Line IRQ VCount`, R10 reload/decrement, HCounter, VCounter, enable/disable, região, VDP status, linha INT do Z80 e reconhecimento de interrupção.

Modelo atual registrado na matriz: VINT pendente HPos20/T13; HINT/R10 HPos21/T14; VCounter HPos23/T15; VBlank/overflow HPos24/T16. MAME 315-5124 diverge em HINT/VINT. São hipóteses testadas internamente, não medições físicas. Não mover boundaries sem teste discriminante.

**Aceite:** testes antes/no/depois do boundary, integração Scheduler/Z80, snapshot/restore e equivalência contínuo/blocos.

### P3 — Scroll e composição raster
Criar fixtures sintéticas para as quatro faixas do Scroll Test: scroll diagonal do fundo, planos/blocos sobrepostos com movimento independente, deslocamentos esquerda↔direita, direção vertical da quarta faixa, prioridade, clipping e writes intralinha.

O relato inclui overlays que não se sobrepõem/não se deslocam e sentido vertical invertido. Separar cálculo de endereço, latch, composição e clipping.

**Aceite:** fixtures determinísticas de scroll multi-faixa, wrap, prioridade e raster writes.

### P4 — Zoom, bordas e pipeline de composição
O Zoom Effect apresentou leve distorção na extremidade direita, ausente/uniforme na referência Ares. Comparar framebuffer lógico RGB888 antes do WGPU; verificar largura ativa, última coluna, stride, índice final, clipping, span final, UV e nearest sampling. Só investigar upload/sampling se o framebuffer lógico estiver correto.

**Aceite:** asserts de última coluna/linha e comparação pré-WGPU.

### P5 — Sprites e pipeline de fetch
Auditar SAT, seleção por scanline, Y/X/tile/atributos, padrões/bitplanes, prioridade, clipping, transparência, deslocamento X, zoom, overflow, colisão e writes intralinha. Separar erro semântico de atributo de ausência de fetch temporizado por dot. O scorecard registra seleção SAT por scanline e leitura de padrões durante composição; não há pipeline completo de fetch por dot.

**Aceite:** testes isolados por atributo e fixtures de sobreposição/prioridade/clipping/overflow.

### P6 — Modos, mapper e perfis de hardware
Validar Mode 4 e legacy/TMS no SMS1 315-5124, preservando o TMS9918A do SG-1000 como hardware separado. Cosmic Spacehead possui mapper Codemasters e requisito de vídeo especial que pode depender de SMS2; não é prova isolada de defeito geral de sprites SMS1. The Smurfs Travel the World (Brazil) ainda requer confirmação de identidade do dump. Andre Agassi é caso de boot, não de sprites.

**Aceite:** matriz de compatibilidade por ASIC, modo, região e mapper; sem fallback silencioso.

### P7 — Boot de cartucho e diagnóstico CPU/mapper/PSG
Andre Agassi Tennis apresentou tela preta e zunido. O log de host anterior confirmou Vulkan/CPAL e pacing aproximado de 60 Hz, mas não continha PC Z80, trace de instruções/barramento, estado do mapper, writes VDP/PSG ou motivo de parada.

Usar trace para separar reset/PC, mapper, RAM/stack, comandos VDP/display enable, PSG, IRQ e WAIT. Não concluir falha Z80 ou PSG pelo sintoma isolado.

**Aceite:** trace mínimo reproduzível e hipótese discriminada por teste automatizado.

### P8 — Diagnósticos externos e compatibilidade
Classificar resultados como PASS, FAIL com caso identificado, INCONCLUSIVE, EXECUTED_NO_ORACLE ou NOT_RUN. ZEXALL V0.12VDP/SDSC não constitui validação Z80 E8. Tela preta ou parada na BIOS não autoriza atribuir falha ao Z80. E8 permanece bloqueado até fechamento formal de E7.

### P9 — Integração e liberação
Executar fmt, Clippy com warnings negados, testes direcionados/workspace, regressões SMS1/SMS2/SG-1000, runner full-machine, determinismo, build Windows x64 Release, Save State/SRAM/áudio/host e comparação de framebuffer lógico. Atualizar scorecards e matriz.

A campanha física/manual permanece pausada por decisão do usuário até progresso interno verificável.

## 3. Agentes especializados

Os perfis em `.github/agents/` são definições customizadas para runtimes compatíveis com agentes (por exemplo, GitHub Copilot/VS Code). **Versionar os perfis não inicia agentes autônomos**; o runtime precisa selecioná-los ou delegar tarefas.

| Perfil | Missão | Limites |
|---|---|---|
| `sms-vdp-data-port` | P1: portas, endereçamento e read-ahead | Não alterar IRQ, scroll, sprites ou Scheduler sem prova |
| `sms-vdp-timing` | P2: IRQ, H/V counters e Scheduler | Não alterar GUI/renderização |
| `sms-vdp-raster` | P3–P5: scroll, zoom, sprites e fetch | Implementação após P1/P2 estáveis |
| `sms-regression-auditor` | Revisão independente e evidências | Não alterar produção nem aprovar a própria mudança |

### Paralelismo controlado
- Um agente implementador por área/arquivos compartilhados.
- Auditor pode trabalhar em paralelo em análise e revisão sem editar os mesmos arquivos.
- Não executar mudanças concorrentes no mesmo `SmsVdp`, barramento ou Scheduler.
- Cada entrega informa hipótese, evidência, arquivos, testes pré/pós, riscos e PR.
- Coordenador integra sequência, resolve conflitos e impede contaminação entre etapas.

## 4. PRs de suporte na fotografia consultada
- #850 — relatório de validação Windows x64 Release; aberta.
- #851 — auditoria do caminho SMS1 ativo; aberta, documental.
- #852 — exportação local de trace SMS; aberta/draft. CI #7439 passou no HEAD `93588d67e78a450e6791965186e32d54c0dc8983`; validação manual Windows e revisão ainda pendentes.
- #853 — campanha de compatibilidade; aberta/draft, documental.

Não assumir que PR aberta está integrada. Revalidar estado, head SHA, reviews e CI antes de merge.

## 5. Critérios gerais de aceite
1. Teste reproduz o defeito na baseline e passa no candidato.
2. Teste observa comportamento funcional, não apenas ausência de panic ou compilação.
3. Regressões de perfis não envolvidos permanecem verdes.
4. Contínuo/blocos e snapshot/restore preservam semântica quando aplicável.
5. CI passou no HEAD exato.
6. Documentação registra evidência, hipóteses descartadas e limites.
7. Nenhuma alegação de precisão física sem medição física.

## 6. Próxima ação executável
Iniciar P1: ler a implementação ativa das portas do `SmsVdp` e os testes existentes; criar dois testes independentes para `1byte wr sets rd adr` e `1byte wr sets wr adr`. Não alterar produção até isolar cada sequência. Registrar estado de endereço e read-ahead em cada passo.

## 7. Referências
- [SMS1 VDP scorecard](../sms1-vdp-completion-scorecard.md)
- [SMS1 temporal evidence matrix](../audits/sms1-vdp-temporal-evidence-matrix-2026-11-14.md)
- [PR #851 — runtime audit](https://github.com/leonardo201800478/sega-emulator/pull/851)
- [PR #853 — compatibility campaign](https://github.com/leonardo201800478/sega-emulator/pull/853)
- [PR #852 — trace export](https://github.com/leonardo201800478/sega-emulator/pull/852)
- [PR #850 — Windows validation](https://github.com/leonardo201800478/sega-emulator/pull/850)
- [Z80 completion scorecard](../z80-completion-scorecard.md)
- [Z80 DD/FD semantic matrix](../z80-dd-fd-semantic-matrix.md)
