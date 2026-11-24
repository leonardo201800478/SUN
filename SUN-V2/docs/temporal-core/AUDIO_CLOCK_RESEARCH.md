# Temporal Core — pesquisa dos clocks de áudio SMS

**Status:** levantamento de domínios; não define ainda a implementação dos chips nem a unidade temporal final.

## 1. PSG SN76489 integrado ao VDP Sega

A referência técnica do SMS Power informa que o PSG recebe um clock de entrada nominal de:
- NTSC: 3.579545 MHz.
- PAL/SECAM: 3.546893 MHz.

O chip divide esse clock por 16 para gerar seu clock interno. Os contadores de tom e ruído avançam nesse domínio dividido. A documentação também descreve diferenças entre versões do SN76489, incluindo a variante Sega com LFSR de 16 bits e comportamento específico de períodos zero/um.

Fonte: [SMS Power — SN76489](https://www.smspower.org/Development/SN76489).

O documento técnico de áudio fornecido no projeto também propõe entrada PSG igual ao clock do Z80 e tick interno dividido por 16. Essa informação é coerente com a referência do SMS Power, mas as demais particularidades descritas naquele documento (forma de onda, polaridade, período zero, modelo analógico e parâmetros de filtros) devem ser auditadas separadamente; não são automaticamente aceitas como fatos do hardware.

## 2. YM2413 / OPLL

O manual de aplicação do YM2413 informa:
- cristal de referência de 3.579545 MHz em XIN/XOUT;
- frequência interna de amostragem indicada como fM/72;
- clock nominal de referência de 3.6 MHz (3.579545 MHz) para AM, vibrato e temporizações de envelope.

Fonte: [YM2413 Application Manual — SMS Power](https://www.smspower.org/maxim/Documents/YM2413ApplicationManual).

As notas de engenharia reversa do SMS Power também usam a relação 3.579545 MHz / 72 em cálculos de AM e PM.

Fontes:
- [YM2413 reverse engineering — AM](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-11-28)
- [YM2413 reverse engineering — PM](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-01).

## 3. Consequência para o modelo temporal

O SMS possui domínios com frequências nominais iguais ou derivadas de uma referência comum:
- Z80: clock de entrada do processador.
- PSG: clock de entrada e divisor interno /16.
- YM2413: clock de entrada e temporizações internas derivadas, incluindo fM/72.
- VDP: clock próprio, com relação documentada ao clock do Z80.

Isso favorece investigar uma base temporal comum inteira, com divisores e fases explícitos, em vez de usar ponto flutuante para avançar cada dispositivo.

Entretanto, não devemos supor que todos os chips sejam ligados ao mesmo sinal físico ou que todas as revisões tenham exatamente a mesma fase. O projeto precisa registrar, por modelo de máquina, fonte de clock, divisor, fase inicial e tolerância conhecida.

## 4. Questões abertas

- O YM2413 no SMS recebe sempre o mesmo clock efetivo do Z80 ou existem variações de placa/revisão?
- Como o clock e o reset do YM2413 se relacionam com a fase do VDP e do Z80?
- O PSG integrado no VDP compartilha fase com a CPU ou apenas frequência nominal?
- Quais transições internas do PSG precisam ser visíveis ao barramento, além de atualizações em suas portas?
- Qual é a menor unidade temporal necessária para representar os efeitos observáveis sem gerar eventos desnecessários por tick?
- Como preservar os divisores internos e a fase em save states e replay?

## 5. Candidato matemático para prototipagem

O valor de 3.579545 MHz × 15 resulta em 53.693175 MHz. O clock VDP de 10.738635 MHz × 5 resulta no mesmo valor. Isso sugere uma possível unidade mestre NTSC:
- 15 ticks mestres por T-state do Z80;
- 5 ticks mestres por clock VDP;
- 240 ticks mestres por tick interno PSG (/16);
- 1080 ticks mestres por período de amostragem YM2413 (fM/72).

Essas relações são apenas uma hipótese de representação matemática. A fase real entre sinais, clocks de revisão de hardware, comportamento PAL e granularidade dos eventos precisam ser confirmados antes de transformar essa unidade em API pública.

**Não foi tomada decisão sobre o tipo inteiro, largura, epoch, wrap/overflow, nem sobre a fila de eventos.**
