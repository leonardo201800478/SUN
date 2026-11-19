# Prompt de continuidade — YM2413 integrado ao Master System japonês

## Autoridade e escopo

- Repositório: https://github.com/leonardo201800478/sega-emulator
- Este arquivo é um roteiro operacional, não uma especificação concorrente. O contrato técnico vive em [ym2413-fm-model.md](ym2413-fm-model.md); a evidência independente M2 permanece em [ym2413-pre-implementation-audit-2026-11-17.md](../audits/ym2413-pre-implementation-audit-2026-11-17.md).
- Responsabilidade do Agente 1: arquitetura, auditoria de fontes, contratos de hardware, documentação, scorecard e coordenação das PRs FM. Não implementar operadores nem síntese PCM.
- Escopo de implementação: YM2413 integrado do perfil físico Japanese Master System MK-2000. O Mark III FM Sound Unit é documentado apenas como contraste de hardware; seu módulo externo não será implementado.
- A trilha SMS-FM é uma característica de hardware do Master System, não uma nova plataforma; pode avançar antes do Game Gear sem alterar a ordem geral de plataformas.

## Leitura obrigatória

- [Modelo arquitetural YM2413](ym2413-fm-model.md)
- [Auditoria M2 independente](../audits/ym2413-pre-implementation-audit-2026-11-17.md)
- [Roadmap de expansões/acessórios](../hardware/sms-expansion-accessories-roadmap.md)
- [Roadmap Modern Hi-Fi de áudio](../audio/modern-hifi-audio-roadmap.md)
- [Histórico de migração do runtime de áudio](native-audio-runtime.md) — documento histórico; o código ativo e o roadmap Modern Hi-Fi prevalecem.
- [ADR do MachineScheduler](adr/0002-machine-temporal-scheduler.md)
- [Modelo de código SMS](master-system-code-model.md)

## Contratos não negociáveis

- Separar modelo físico, território do software e `FmMode { Auto, Off, On }`. Território não prova presença de YM2413.
- `Auto` exige perfil físico com chip integrado e correspondência exata de título/revisão/hash no catálogo, com driver/dados FM comprovados. Sem correspondência, FM permanece desativado.
- `On` ignora o catálogo apenas se o perfil físico já possuir YM2413 integrado; não cria hardware em Mark III sem unidade ou Export SMS sem FM de fábrica e não fabrica writes/dados FM.
- `Off` silencia a saída; não interrompe estado/clock interno sem evidência de clock-gating.
- Japanese Master System MK-2000 permite PSG+FM simultâneos por $F2. Mark III + FM Sound Unit é mutuamente exclusivo e fica fora da implementação. Mark III sem unidade e Export SMS de fábrica não recebem FM por suposição.
- $F2 readback japonês: D7–D5 são os bits 11/7/3 do contador C-Sync; D4 é zero; D3–D0 devolvem o último nibble escrito. Na escrita, só D1 (mute PSG) e D0 (mute FM) controlam roteamento. Contrato DOCUMENTADO no [SMS Power Audio Control Port](https://www.smspower.org/Development/AudioControlPort), mas teste físico por revisão continua PENDENTE. Não inferir contador a partir de VCounter, scanline ou timer do host.
- Clock/fase comum PSG/FM permanece **PENDENTE** até evidência física. Reutilizar o avanço do PSG somente se FM-A01 confirmar a conexão; caso contrário, registrar domínio explícito no mesmo MachineScheduler.
- Preservar MO/RO separadas até confirmar a rede analógica da placa. Não transplantar circuito MSX/SFG-05.
- Core 100% Rust, independente de SMS/audio_runtime/GUI/host; hot path sem alocações dinâmicas ou locks; não copiar código/tabelas GPL.
- MachineScheduler é o único dono do tempo. GUI apenas configura/apresenta; nenhum timer do host avança hardware.

## Limites atuais de integração

- O manual Yamaha documenta 12 clocks após escrita de endereço e 84 após escrita de dados. O efeito de violar a janela permanece PENDENTE.
- O mixer aceita fontes alinhadas de mesmo tamanho, mas a sessão atual recebe uma fonte por chamada. Não enviar PSG e FM em chamadas sequenciais: isso concatena blocos.
- A taxa nominal YM2413 (3.579.545/72 ≈ 49.715,9027778 Hz) deve ser alinhada deterministicamente à base interna de 48 kHz antes da sessão atual, ou o mixer deve receber fontes com timestamps/taxas explícitos.
- Snapshot/replay futuro deve incluir latch, registradores, deadlines, slot/fase, PG/EG/LFO/feedback, MO/RO, roteamento e o estado de conversão/mixagem que afete determinismo.

## Coordenação live das PRs — snapshot não substitui consulta remota

1. **PR #994:** integrada em main pelo squash `faecfc7fc6a869884bf5aaad063473ca5002be0d`. Preservar o arquivo M2 como evidência independente; não duplicar seu conteúdo integral em outros documentos.
2. **Main pós-merge:** workflow CI `44213210752` para `faecfc7fc6a869884bf5aaad063473ca5002be0d` concluiu SUCCESS. Reconsultar main HEAD e checks em cada execução.
3. **PR #996:** consolida a fundação antes dividida entre #993/#995, diretamente sobre main, no crate independente `native/ym2413`. A branch atual deve ser consultada live; qualquer atualização documental cria novo SHA e exige CI novo.
4. **PRs #993 e #995:** fechadas sem merge e substituídas pela #996. Não reabrir nem integrar separadamente, salvo nova evidência de regressão.
5. A integração da #996 está autorizada após revisão técnica, diff final, lockfiles coerentes e todos os gates aplicáveis PASS no HEAD exato. Manter Draft enquanto CI/revisão estiverem pendentes; não transferir checks de SHA anterior.

## Separação dos gates

- **Pode iniciar síntese digital pura:** PG/EG/AM/PM/feedback e pipeline podem avançar em incrementos isolados, com modelos candidatos, vetores e incertezas explícitas. FM-A01/A02/A03/A04/A05/A07 são bloqueios de integração física/áudio, não do core digital puro.
- **Bloqueia fidelidade digital:** estados iniciais, offsets de pipeline, detalhes PG/EG/feedback, presets originais e comportamento de writes fora de 12/84 exigem vetores/diferenciais antes de PASS.
- **Bloqueia integração SMS:** FM-A01 clock/fase; FM-A02 rede MO/RO; FM-A03 C-Sync/$F2; FM-A04 perfil de placa; FM-A05 catálogo Auto; FM-A06 janela de writes; FM-A07 mixer/resampler/save state.
- Não declarar cycle-accurate, compatibilidade de ROM ou aceite físico sem evidência independente no escopo específico.

## Próximas etapas do Agente 1

1. Consultar main, PR #996, branches, diffs, reviews, comentários, ahead/behind e CI no SHA exato.
2. Revisar o diff consolidado: crate sem dependências, dependência unidirecional `audio_runtime → sega-ym2413`, workspace e lockfiles coerentes.
3. Atualizar o scorecard em [ym2413-fm-model.md](ym2413-fm-model.md) com os resultados reais do HEAD corrente; checks antigos não são transferíveis.
4. Integrar a #996 somente quando todos os gates aplicáveis PASS; após merge, confirmar main HEAD e workflow pós-merge no SHA exato.
5. Deixar a implementação de operadores/síntese para o agente responsável por esse escopo, usando o contrato e scorecard canônicos.

SKIPPED, pending, cancelled, ausente e NOT RUN nunca são PASS.