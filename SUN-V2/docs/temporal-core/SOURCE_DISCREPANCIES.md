# Temporal Core — divergências e qualidade das fontes

**Status:** registro ativo de conflitos encontrados durante a pesquisa. Nenhuma divergência deve ser resolvida por suposição.

## 1. PSG: mapa de portas I/O

O guia de plataformas do 8bitworkshop lista `$F0-$F2` como portas de som/PSG.

Já a documentação técnica específica do VDP e as referências SMS Power descrevem:
- `$7E`: V counter na leitura / PSG na escrita;
- `$7F`: H counter na leitura / PSG na escrita (espelho);
- `$BE`: VDP data;
- `$BF`: VDP control.

Fontes:
- [8bitworkshop — Sega Master System](https://8bitworkshop.com/docs/platforms/sms/)
- [SMS Power — SMS Official Docs](https://www.smspower.org/Development/SMSOfficialDocs)
- [Charles MacDonald — Sega Master System VDP documentation](https://github.com/franckverrot/EmulationResources/blob/master/consoles/sms-gg/Sega%20Master%20System%20VDP%20documentation.txt)

**Tratamento:** não usar o quadro de portas do 8bitworkshop como especificação de hardware. Priorizar documentação primária/mais específica e confirmar por testes de portas e ROMs de diagnóstico.

## 2. Frequência nominal do clock NTSC

O manual de serviço Sega registra 10.738635 MHz no pino 30 do IC5/VDP (±100 Hz) e 3.579545 MHz no pino 34, ligado ao Z80 (±30 Hz).

A página Clock Rate do SMS Power descreve o cristal mestre como aproximadamente 53.6931 MHz e a divisão por 15 como aproximadamente 3.579540 MHz. A diferença aparente decorre de arredondamento e da distinção entre frequência do oscilador e valor oficial da subportadora.

Fontes:
- [Sega Master System Service Manual](https://segaretro.org/images/7/7b/SMSServiceManualEU.pdf)
- [SMS Power — Clock Rate](https://www.smspower.org/Development/ClockRate)

**Tratamento:** registrar valor, ponto de medição, tolerância e derivação. Não misturar frequência real do oscilador, subportadora e valor arredondado.

## 3. PAL europeu: 3.546893 versus 3.546895 MHz

SMS Power apresenta 3.546893 MHz como clock PAL/SECAM e também deriva 3.546895 MHz a partir da frequência nominal da subportadora. MAME usa `53.203425 MHz / 15 = 3.546895 MHz`. A referência de serviço PAL reproduzida por fonte técnica secundária cita aproximadamente 10.640684 MHz no VDP e 3.546893 MHz no Z80.

Fontes:
- [SMS Power — Clock Rate](https://www.smspower.org/Development/ClockRate)
- [MAME sms.cpp](https://github.com/mamedev/mame/blob/master/src/mame/sega/sms.cpp)
- [Sega Master System Troubleshooting Guide — referências ao manual](https://wiki.retrotechcollection.com/Sega_Master_System_Troubleshooting_Guide)

**Tratamento:** manter ambos como candidatos até definir a convenção de clock e a tolerância. A diferença não deve ser apagada por arredondamento silencioso.

## 4. PAL-M brasileiro: runtime V1 versus modelo MAME

O worker V1 seleciona PAL-M para Brazil, mas fornece 3.579545 MHz como clock do sistema/PSG.

MAME define `MASTER_CLOCK_PALM = 10.726833 MHz` (comentando XTAL nominal 10.726834 MHz), configura Z80 a `MASTER_CLOCK_PALM/3 = 3.575611 MHz`, VDP a `MASTER_CLOCK_PALM` e parâmetros de tela NTSC. O comentário MAME diz que 10.726833 é usado em vez do nominal porque o valor nominal provoca erros no Flubba VDPTest, provavelmente por arredondamento do core.

Fonte: [MAME sms.cpp](https://github.com/mamedev/mame/blob/master/src/mame/sega/sms.cpp), `MASTER_CLOCK_PALM`, `sms_br_base`, `sms1_br`.

**Conflito material:** 3.579545 MHz (V1) versus 3.575611 MHz (modelo MAME PAL-M). Não é diferença de arredondamento; são aproximadamente 3.934 kHz.

**Tratamento:** UNKNOWN para o hardware TecToy-alvo. Obter manual/esquema de serviço PAL-M ou medir VDP input e Z80/PSG clock em placa identificada. Não alterar V1 nem fixar V2 com base exclusiva no MAME.

## 5. PSG: período de tom zero/um

A especificação de áudio anexada ao projeto afirma que N=0 se comporta como N=1 e pode produzir portadora ultrassônica de aproximadamente 111.86 kHz ou nível alto.

SMS Power afirma que, quando o registrador de período vale 0 ou 1, a saída fica em nível constante +1; a página também descreve o uso desse comportamento em sample playback.

Fonte: [SMS Power — SN76489](https://www.smspower.org/Development/SN76489).

**Tratamento:** divergência de alta prioridade. Separar N=0 de N=1 e criar teste que observe saída digital e espectro analógico. Não assumir alternância a 111.86 kHz.

## 6. Modelo analógico de áudio

O documento de áudio anexado propõe filtros de 35 Hz/12 kHz, oversampling e soft clipping `tanh`. O manual de serviço Sega consultado informa saída no pino 10 do VDP, máximo de 300 mVp-p e banda de 20 Hz–20 kHz, além de listar componentes da rota de áudio; não fornece os cortes IIR ou uma função de transferência equivalente a `tanh`.

Fonte primária: [Sega Master System Service Manual](https://segaretro.org/images/7/7b/SMSServiceManualEU.pdf), página impressa 8 e esquemas/listas de peças.

**Tratamento:** valores de DSP permanecem hipóteses. Identificar revisão da placa e medir resposta antes de adotá-los como modelo de hardware.

## 7. Limite de precisão da documentação VDP

A documentação de Charles MacDonald descreve counters, linhas, flags e condições de IRQ, mas registra que parte dos dados deriva do manual TMS9918 e de testes em Genesis NTSC, alertando que um SMS real pode divergir.

Fonte: [Charles MacDonald — Sega Master System VDP documentation](https://github.com/franckverrot/EmulationResources/blob/master/consoles/sms-gg/Sega%20Master%20System%20VDP%20documentation.txt).

**Tratamento:** usar como base de comportamento, não como prova de microtiming completo. Complementar com MAME/ares/GX, ROMs de diagnóstico e observação de hardware.

## 8. Regras de precedência das fontes

1. Datasheet/manual do fabricante e manual de serviço Sega.
2. Esquema e medição de hardware com revisão de placa identificada.
3. Documentação técnica específica do hardware com autor e limitações identificados.
4. Testes de hardware/ROMs de diagnóstico com resultado reproduzível.
5. Código de emuladores conceituados, estudado na revisão/commit correspondente.
6. Guias de desenvolvimento e resumos gerais.
7. Hipóteses derivadas do modelo matemático.

Código de emulador é evidência de uma implementação, não prova automática de comportamento físico. Quando fontes divergem, registrar o conflito e criar um teste que possa discriminá-las.
