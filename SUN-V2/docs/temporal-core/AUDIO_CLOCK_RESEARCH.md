# Temporal Core — pesquisa dos clocks de áudio SMS

**Status:** pesquisa R0 concluída; ver parecer R0_CLOSEOUT.md. Este documento não define a implementação dos chips, a unidade temporal ou a fase inicial.

## 1. PSG SN76489 integrado ao VDP Sega

SMS Power informa que o PSG recebe clock de entrada nominal:
- NTSC: 3.579545 MHz;
- PAL/SECAM: 3.546893 MHz (a página também deriva 3.546895 MHz a partir da subportadora PAL).

O clock interno é dividido por 16. Os contadores de tom e ruído avançam nesse domínio dividido. A variante Sega integrada usa LFSR de 16 bits, distinta de algumas versões discretas.

Fonte: [SMS Power — SN76489](https://www.smspower.org/Development/SN76489).

### PAL-M

Não escolher automaticamente entre clock NTSC e PAL-M para Brasil. MAME configura VDP a 10.726833 MHz e Z80/PSG a 3.575611 MHz; V1 usa 3.579545 MHz. A placa TecToy original usa MC1377P, e a subportadora PAL-M de 3.575611 MHz pode ser gerada no encoder separadamente do clock CPU/PSG. Há relatos comunitários de XTAL 10.726834 MHz e também uma marcação de 10.746168 MHz não confirmada. Portanto, 3.575611 MHz é candidato do modelo MAME e do padrão de cor, não fato físico fechado para PSG da TecToy. Fontes: [SMS Power board](https://www.smspower.org/Development/BrazilianMasterSystemTECTOYPOWERBASEPAL-M), [SMS Power conversion thread](https://www.smspower.org/forums/15865-TectoySms1NTSCConversion), [Neto Games](https://www.neto-games.com.br/hardware/sms_tt_cxa1645.php).

Fonte: [MAME sms.cpp](https://github.com/mamedev/mame/blob/master/src/mame/sega/sms.cpp), `MASTER_CLOCK_PALM` e `sms_br_base`.

### Taxas internas derivadas

| Perfil candidato | Entrada PSG | Divisor | Taxa interna |
|---|---:|---:|---:|
| NTSC | 3.579545 MHz | /16 | 223,721.5625 Hz |
| PAL, valor SMS Power | 3.546893 MHz | /16 | 221,680.8125 Hz |
| PAL, derivação MAME/subportadora | 3.546895 MHz | /16 | 221,680.9375 Hz |
| PAL-M, modelo MAME | 3.575611 MHz | /16 | 223,475.6875 Hz |

Essas taxas são derivadas de candidatos de clock, não medições independentes do divisor interno.

## 2. Particularidade crítica: período de tom 0/1

SMS Power afirma que, quando o registrador de período de tom vale 0 ou 1, a saída fica em nível constante +1; o documento também observa o uso dessa condição para sample playback.

Fonte: [SMS Power — SN76489](https://www.smspower.org/Development/SN76489).

O código V1 congelado também trata `period <= 1` como nível constante alto em `tick_internal_clock()`; portanto, a implementação V1 e a descrição SMS Power concordam nesse ponto. O documento técnico de áudio fornecido no projeto menciona uma portadora ultrassônica de aproximadamente 111.86 kHz ou saída efetivamente travada em nível alto; somente a segunda alternativa é sustentada pela documentação consultada e pelo código V1.

**Tratamento:** a afirmação “constante +1” é a referência documental atual; a alegação de portadora deve ser tratada como divergência e não implementada sem teste discriminante no VDP Sega. Testar N=0 e N=1 separadamente, com captura digital/analógica e análise espectral, incluindo playback PCM de jogos reais.

## 3. Ruído e LFSR Sega

SMS Power descreve para SMS1/SMS2:
- LFSR de 16 bits;
- modo “periodic”: realimentação do bit 0, sequência de 16 estados úteis;
- escrita no registrador de ruído reinicia o shift register com todos os bits zero, exceto o bit mais alto;
- variante Sega de white noise usa taps nos bits 0 e 3 (feedback equivalente à máscara 0x0009, conforme convenção de deslocamento).

Fonte: [SMS Power — SN76489](https://www.smspower.org/Development/SN76489).

A especificação de áudio anexada ao projeto apresenta a mesma largura, reset 0x8000 e taps 0/3. A equivalência de máscara depende da direção e da convenção de estado/saída do LFSR; validar por sequência completa, não somente pelo polinômio.

## 4. YM2413 / OPLL

O manual de aplicação do YM2413 descreve:
- clock de referência de 3.579545 MHz;
- temporização de amostras derivada de fM/72;
- clocks de LFO/AM/vibrato e envelopes derivados internamente.

Fontes:
- [YM2413 Application Manual — SMS Power](https://www.smspower.org/maxim/Documents/YM2413ApplicationManual)
- [YM2413 reverse engineering — AM](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-11-28)
- [YM2413 reverse engineering — PM](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-01)

MAME instancia o YM2413 do Japanese SMS com `XTAL(10'738'635)/3` = 3.579545 MHz.

Fonte: [MAME sms.cpp](https://github.com/mamedev/mame/blob/master/src/mame/sega/sms.cpp), função `smsj`.

A relação nominal não prova a fase absoluta entre YM2413, Z80 e VDP, nem o comportamento de reset em todas as placas. O V2 deve preservar clock e fase como estado explícito.

## 5. Pipeline analógico pós-chip

O manual de serviço Sega documenta:
- saída de áudio no pino 10 do IC5/VDP;
- nível máximo indicado de 300 mVp-p;
- banda indicada de 20 Hz a 20 kHz;
- componentes discretos na placa e transistores/capacitores/resistores associados à rota de áudio.

Fonte primária: [Sega Master System Service Manual (PDF)](https://segaretro.org/images/7/7b/SMSServiceManualEU.pdf), seção de diagnóstico “picture appears alone without sound”, página impressa 8, e listas de peças/esquemas.

O documento técnico anexado ao projeto propõe filtros IIR de 35 Hz e 12 kHz, capacitor de acoplamento, oversampling a 223.72 kHz e soft clipping `tanh`. Esses parâmetros são uma hipótese de modelagem, não valores confirmados pelo manual consultado. A lista de componentes e o limite de banda não bastam para inferir a função de transferência da placa.

**Tratamento:** separar:
1. saída digital do PSG;
2. saída elétrica do VDP;
3. circuito analógico da revisão da placa;
4. cabo/entrada da TV;
5. filtro de reconstrução escolhido pelo emulador.

Antes de fixar coeficientes DSP, identificar revisão de placa, esquema e valores, calcular resposta teórica e comparar captura física. Não usar `tanh` como substituto genérico de transistor sem curva medida.

## 6. Consequência para o modelo temporal

Domínios candidatos:
- Z80: clock de entrada da CPU;
- PSG: clock de entrada e divisor /16;
- YM2413: clock de entrada e temporizações internas derivadas, incluindo fM/72;
- VDP: clock de entrada próprio, relacionado ao Z80 conforme perfil.

A arquitetura deve manter o estado digital e os divisores internos sob autoridade temporal única. O resampler e os filtros de saída não podem avançar clocks de chip.

## 7. Questões abertas

- PAL-M: qual clock real de PSG/Z80 nas placas TecToy-alvo?
- PAL: adotar 3.546893 ou 3.546895 MHz, e segundo qual convenção?
- PSG: qual comportamento físico/digital exato de N=0 e N=1 no VDP integrado?
- PSG: fase inicial dos divisores e nível de saída após reset?
- YM2413: fase do clock/reset em relação ao VDP e Z80?
- Qual resposta analógica real pertence a cada revisão de placa SMS?
- Quais filtros são do hardware e quais são apenas reconstrução digital do emulador?

Nenhuma destas decisões está fechada nesta pesquisa.
