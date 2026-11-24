# Inventário crítico V1 → V2

**Projeto:** SUN V2  
**Escopo:** primeira máquina SMS  
**Status:** inventário inicial baseado em documentação e fontes V1 identificadas; exige reconfirmação no SHA de referência congelado antes de qualquer decisão de implementação.  
**Regra:** este documento registra conhecimento transferível, não autoriza cópia de código.

## 1. Conclusão executiva

A V1 já resolveu uma parte grande do problema funcional: há CPU Z80, barramento/memória/cartucho, VDP SMS, PSG SN76489, runtime e pipeline de áudio. O que o V2 precisa mudar não é simplesmente reescrever esses componentes com nomes novos.

A principal diferença é fazer com que os componentes nasçam sob contratos temporais e de observabilidade explícitos. A maturidade funcional de um componente não prova que sua fronteira temporal esteja correta para interação com os demais.

O estado conhecido, sujeito à reconfirmação no HEAD V1, é:

- **CPU:** cobertura funcional ampla; o README do crate compartilhado descreve `Z80::step` como execução de instrução completa, retornando T-states nominais. Os hooks de barramento citados são diagnósticos e não suspendem a execução nem despacham eventos. Portanto, não tratar isso como microsequenciador temporal concluído.
- **VDP:** renderer SMS funcional e migrado para `sega_video::sms`; há renderização por scanline/span e boundaries subline seletivos documentados. A documentação não declara dot-accuracy nem pipeline completo de fetch por dot.
- **PSG:** core SN76489 digital compartilhado, com clock/divisor, estado de canais, geração de amostras e filtros. É uma boa referência de comportamento e custo, mas sua interface V2 deve explicitar tempo de entrada, fase e saída determinística por intervalo.
- **YM2413:** existe auditoria técnica extensa, mas as decisões de perfil de placa, clock/fase, C-Sync, $F0–$F2, MO/RO e continuidade do snapshot ainda condicionam a integração. Não assumir FM operacional no runtime V1.
- **Scheduler:** há infraestrutura temporal e coordenação de máquina. A existência do Scheduler não torna automaticamente todos os ciclos da CPU, fases do barramento e eventos de dispositivo temporalmente completos.
- **Áudio host:** mixer, sessão, resampler e sink estão separados do core de chip. O V2 deve manter essa separação e formalizar timestamps/alinhamento multi-fonte.

## 2. Matriz por subsistema

| Subsistema | Conhecimento V1 aproveitável | Lacuna que importa ao V2 | Decisão V2 |
|---|---|---|---|
| Z80 | Estado arquitetural, semântica de instruções, flags, prefixos e regressões acumuladas | Executor por instrução não representa automaticamente suspensão em fronteiras de barramento | Reimplementar independentemente; começar com semântica mínima correta e expandir por matriz; microsequenciar apenas operações com observação temporal |
| Barramento | Mapas, decodificação, portas, transações já conhecidas e casos de integração | Precisa de timestamp, fase, espera, retomada e exatamente-uma-vez como contrato explícito | Construir contrato mínimo antes de expandir dispositivos |
| Memória/cartucho | Mapa SMS, RAM, ROM e comportamento de mapper já estudado | Catálogo e persistência de mappers precisam ser selecionados por escopo de jogos | Começar com ROM e RAM mínimas; adicionar mapper por compatibilidade demonstrada |
| VDP SMS | Mode 4, VRAM/CRAM, portas, registradores, raster, sprites, IRQ e testes | Scanline/span mais refinamentos seletivos não equivale a dot-accuracy; vários latches/fetches permanecem aproximados | Implementar perfil SMS1 mínimo, raster determinístico e boundaries observáveis; ampliar por testes |
| PSG SN76489 | Core digital, divisor, canais tone/noise, atenuação, buffer e filtragem | Contrato de avanço/saída precisa ser temporalmente explícito e independente do host | Primeira fonte de áudio; core puro com avanço por clocks/timestamps e testes de chunking |
| YM2413 | Pesquisa de manual, engenharia reversa e auditoria de placa | Clock/fase de placa, C-Sync, roteamento, saídas e integração de snapshot ainda têm lacunas | Não bloquear primeira fatia; implementar core separado após fechar contrato de áudio e evidência de placa |
| Mixer/resampler | Sessão Rust, mixer e conversão para taxa do host | Fontes precisam estar alinhadas e seu estado de conversão deve ser restaurável | Projetar desde o início interface multi-fonte com timestamp e fase; não acoplar ao CPAL |
| Scheduler | Tempo virtual, domínios, eventos, snapshot e testes já explorados | A API final precisa ser provada com a semântica e granularidade exigidas pelo SMS V2 | CHRONOS independente; não copiar estrutura de dados antes dos protótipos |
| Host | Experiência prévia com worker, mailbox, GUI, vídeo e áudio | Filas e pacing podem mascarar latência e não podem comandar clock virtual | Fora da primeira fatia; integrar após runtime headless determinístico |

## 3. Fatos, limites e grau de confiança

### 3.1 CPU Z80

A documentação do crate `native/z80_cpu` afirma que `Z80::step` executa uma instrução completa e retorna T-states nominais. Os hooks de observação do barramento são diagnósticos e não suspendem a instrução nem despacham eventos do Scheduler.

**Confiança:** alta para o contrato descrito no README do crate consultado.  
**Implicação:** V2 deve separar semântica funcional de instrução e transações temporais do barramento. Não precisa começar com micro-operação para toda instrução; precisa permitir suspensão onde a interação com hardware exigir.

### 3.2 VDP SMS

A documentação de arquitetura registra o engine SMS compartilhado em `sega_video::sms`, com portas, VRAM/CRAM, modos, sprites, status, counters e IRQs. O caminho temporal SMS1 tem granularidade scanline/span e boundaries seletivos, mas não modela todos os fetches e latches em dot.

**Confiança:** alta para os limites explicitamente documentados; média para detalhes que dependem de diagnóstico comparativo e ainda não foram verificados em hardware físico.  
**Implicação:** V2 deve preservar a ideia de precisão seletiva, sem herdar automaticamente timestamps ou fases ainda aproximados.

### 3.3 PSG e áudio

O core SN76489 é independente de GUI/PyO3 e mantém clock/divisor e geração digital de amostras. A sessão de áudio separa mixer, DSP, resampler e sink host. A auditoria YM2413 descreve requisitos de mistura PSG+FM, taxa comum e estado de conversão.

**Confiança:** alta para a separação estrutural documentada; média para equivalência acústica com placa física, que exige evidência e medição.  
**Implicação:** começar com PSG puro; deixar a arquitetura pronta para múltiplas fontes, mas não ativar FM por hipótese.

### 3.4 Scheduler e política de precisão

A documentação do Scheduler estabelece que dispositivos podem avançar em blocos e que eventos devem representar ocorrências relevantes, não cada pino/pixel/fase. A ADR-0016 define precisão temporal prioritária para caminhos observáveis de gameplay e atomicidade funcional para o restante.

**Confiança:** alta como diretriz arquitetural registrada na V1; sua adequação ao V2 será reavaliada, mas é consistente com a proposta de híbrido mínimo.  
**Implicação:** V2 não deve perseguir precisão elétrica universal nem criar eventos por tick sem consumidor demonstrado.

## 4. O que pode ser transferido da V1

Transferir como conhecimento, após verificar proveniência e validade:

- mapas de memória e I/O SMS;
- listas de registradores e comportamento funcional;
- documentação de CPU, VDP, PSG e YM2413;
- vetores e ROMs de diagnóstico legalmente utilizáveis;
- casos de regressão reproduzíveis;
- resultados de comparação com MAME, ares, Gearsystem e outras referências;
- perfis de benchmark e metodologia;
- erros conhecidos e explicações já demonstradas.

Cada item deve registrar fonte, SHA/versão, licença quando aplicável, método de validação e limitações.

## 5. O que não será herdado automaticamente

- implementação de CPU, VDP, PSG, Scheduler ou barramento;
- estruturas de dados, APIs ou divisão de crates;
- decisões de fase temporal sem evidência suficiente;
- testes que usem a própria implementação V1 como único oráculo;
- resultados de CI antigos como prova de qualidade V2;
- suposições de compatibilidade de jogos que não tenham ROM/repro/resultado associado.

A V1 pode ser usada como comparador diferencial, mas uma divergência não prova automaticamente que o V2 está errado ou que a V1 está certa.

## 6. Lacunas que devem fechar R0

| ID | Pergunta | Evidência necessária | Prioridade |
|---|---|---|---|
| INV-01 | Qual SHA V1 será congelado como baseline de referência? | HEAD e commit de cada fonte | Bloqueante |
| INV-02 | Quais contratos do Z80 são semânticos e quais são temporais? | README, código, testes e traces | Bloqueante |
| INV-03 | Quais operações de barramento SMS precisam de suspensão/WAIT? | Manual Z80, hardware SMS e casos discriminantes | Bloqueante |
| INV-04 | Quais boundaries VDP são observáveis por jogos e diagnósticos? | Matriz de evidências e ROMs de teste | Alta |
| INV-05 | Quais clocks e fases CPU/VDP/PSG são confirmados por perfil NTSC/PAL? | Fonte primária, cálculo e teste | Bloqueante |
| INV-06 | Qual é a saída de referência do PSG para testes determinísticos? | Vetores/PCM com parâmetros e proveniência | Alta |
| INV-07 | Quais requisitos de placa habilitam YM2413 no Japanese SMS? | Clock/fase, C-Sync, $F0–$F2, MO/RO e perfil | Posterior à primeira fatia |
| INV-08 | Qual workload fixa a baseline de performance? | ROM/hash, perfil, host e protocolo headless | Bloqueante |
| INV-09 | Quais resultados V1 podem ser usados como comparação independente? | Proveniência e oráculo alternativo | Alta |

## 7. Fontes internas consultadas

- [README do Z80 compartilhado](https://github.com/leonardo201800478/SUN/blob/main/native/z80_cpu/README.md)
- [Arquitetura e limites do VDP SMS](https://github.com/leonardo201800478/SUN/blob/main/docs/architecture/sms-vdp.md)
- [Implementação compartilhada SN76489](https://github.com/leonardo201800478/SUN/blob/main/native/sn76489/src/psg.rs)
- [Sessão nativa de áudio](https://github.com/leonardo201800478/SUN/blob/main/native/audio_runtime/src/session.rs)
- [Auditoria pré-implementação YM2413](https://github.com/leonardo201800478/SUN/blob/main/docs/audits/ym2413-pre-implementation-audit-2026-11-17.md)
- [Documentação do Scheduler](https://github.com/leonardo201800478/SUN/blob/main/docs/architecture/scheduler/README.md)
- [ADR-0016 — fidelidade observável e precisão proporcional](https://github.com/leonardo201800478/SUN/blob/main/docs/architecture/adr/0016-observable-fidelity-and-proportional-timing.md)

## 8. Limite deste inventário

Este documento é uma fotografia inicial de documentação e fontes consultadas. Não substitui uma auditoria do SHA V1 congelado, não declara TEMPORAL_PASS e não autoriza copiar componentes para o V2.

Antes de fechar R0, registrar o SHA exato da V1, reconfirmar cada linha da matriz e anexar evidências executáveis ou reproduzíveis.
