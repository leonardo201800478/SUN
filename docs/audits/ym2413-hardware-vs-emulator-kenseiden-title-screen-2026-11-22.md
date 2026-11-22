# Comparação YM2413 — hardware SMS japonês vs SUN / Kenseiden

**Data:** 2026-11-22  
**Escopo:** gravação física do título de Kenseiden versus captura PCM do SUN após 1.800 frames.  
**Resultado:** semelhança temporal/espectral relevante; diferença grande de nível; calibração absoluta bloqueada pela cadeia analógica/captura e pelo headroom.

## 1. Arquivos e proveniência

| Fonte | Formato | Taxa | Canais | Duração | SHA-256 |
|---|---|---:|---:|---:|---|
| Hardware: `01. Title Screen.flac` | FLAC, PCM 24-bit | 44.100 Hz | 2 | 17,700 s | `c165e0ea1717368a8e93d0211cdd87c14d96b15feea8ebd01f0cd71c10c7b076` |
| SUN: `kenseiden-japan-mixed-output.wav` | WAV, float32 | 48.000 Hz | 1 | 30,055 s | `1d6706cd8e52fa52c6071e04b0921054d6917ffaa4d6e8b38b5f68ea0d6c2417` |
| SUN: `kenseiden-japan-fm-source.wav` | WAV, float32 | 48.000 Hz | 1 | 28,134 s | `81a240a1d26bf66c9e006722e1cbba1974fb734a8978af849b8911397c81cd7f` |

A correlação L/R do arquivo físico é **0,99996**; downmix para mono é adequado nesta amostra. A captura mista do SUN foi gerada por `NativeAudioSession::new(48_000, 48_000, 100, sink)`: volume da sessão em **100%**, sem ganho FM adicional. A fonte FM é capturada antes do mixer/sessão.

## 2. Alinhamento temporal

Foi usada representação log-mel de 48 bandas (janela 2.048, hop 480) e alinhamento DTW por distância cosseno. O trecho correspondente do SUN começa aproximadamente em **6,43 s** da captura de 30,055 s.

- Similaridade cosseno média das bandas mel normalizadas: **0,900**; mediana: **0,940**.
- Regressão linear do mapeamento de frames: razão **1,00037** entre os eixos temporais.
- Não é comparação bit a bit nem valida fase, latência de pino ou clock de chip.

As trajetórias espectrais são semelhantes e não mostram diferença média de velocidade relevante neste trecho. DTW absorve pequenas diferenças locais e não prova, isoladamente, exatidão de clock.

## 3. Nível e headroom

| Métrica | Hardware | SUN, trecho alinhado |
|---|---:|---:|
| RMS | 0,19455 | 0,01703 |
| RMS (dBFS) | -14,22 dBFS | -35,37 dBFS |
| Pico | 1,00000 | 0,14868 |
| Pico (dBFS) | 0,00 dBFS | -16,55 dBFS |
| Fator de crista | 14,22 dB | 18,82 dB |
| DC médio | -0,0000049 | +0,004514 |

A diferença global de RMS é **21,15 dB**. A mediana das razões RMS em frames alinhados de 10 ms é **+20,56 dB** (fator linear **10,67**); p10/p90 = **+18,13/+23,00 dB**.

O WAV do harness já estava em volume 100%. O slider do desktop é escala linear aplicada pelo resampler e não pode, sozinho, compensar esta diferença.

### Limite de ganho linear

Aplicar +20,56 dB diretamente ao SUN levaria o pico estimado a **1,586** (+4,0 dBFS); cerca de **3,34%** das amostras ultrapassariam [-1,+1]. Não é correto elevar simplesmente o ganho fixo até igualar RMS: isso causaria clipping substancial.

O arquivo físico também tem amostras no teto digital: aproximadamente **0,0465%** das amostras mono têm módulo >= 0,9999 e **0,0113%** atingem/excedem 1,0. É compatível com saturação no estágio de captura/ADC ou ganho de entrada excessivo. O pico 0 dBFS não é medida calibrada de tensão analógica.

## 4. Diferença espectral

Espectros médios dos frames pareados por DTW; valores SUN menos hardware, antes de compensar o nível global:

| Banda | SUN - hardware |
|---|---:|
| 40–100 Hz | -21,75 dB |
| 100–250 Hz | -21,85 dB |
| 250–500 Hz | -19,47 dB |
| 500 Hz–1 kHz | -19,93 dB |
| 1–2 kHz | -20,22 dB |
| 2–4 kHz | -19,73 dB |
| 4–8 kHz | -18,94 dB |
| 8–12 kHz | -18,14 dB |
| 12–18 kHz | -16,39 dB |

Após remover a diferença global de aproximadamente 20,56 dB, o SUN mantém relativamente mais energia nas bandas altas: cerca de **+4,2 dB** em 12–18 kHz em relação a 40–100 Hz. Isso pode decorrer da ausência de resposta analógica equivalente no PCM, da cadeia de gravação física ou de diferenças na síntese. Não basta para escolher EQ definitivo.

O DC médio do SUN é positivo (+0,004514), enquanto o arquivo físico está praticamente centrado em zero. Investigar na fronteira digital/analógica; não remover offset dentro do núcleo YM2413 sem estabelecer a semântica correta.

## 5. Interpretação

1. A diferença de nível existe nos arquivos: SUN está aproximadamente **20,6–21,2 dB** abaixo do arquivo físico na passagem comparada.
2. Não é falta de volume no slider: o WAV do harness foi produzido com volume 100%.
3. Ganho fixo não resolve RMS e pico simultaneamente. Igualar RMS excede headroom; igualar apenas pico deixa a média cerca de 4,6 dB abaixo.
4. O espectro SUN é relativamente mais brilhante acima de 4 kHz. Pode ser resposta analógica/captura, não necessariamente operador digital.
5. A gravação física é mono na prática e tem clipping de entrada; não fornece calibração absoluta de tensão nem prova a resposta do circuito.
6. A comparação não separa MO/RO, PSG/FM físicos ou resposta do amplificador/ADC. O harness confirma FM não silencioso; a captura física é a soma da saída analógica final.

## 6. Decisão

**Não alterar ainda ganho de produção, escala do operador, mixer global ou limite do resampler.**

+20,56 dB é calibração relativa a este arquivo, não calibração elétrica do YM2413. Aplicar diretamente causa clipping. Não introduzir compressor/limiter para mascarar a diferença sem demonstrar que tal processamento existe na cadeia de referência.

Próximo gate:
- nível de entrada fixo e documentado;
- sem normalização, AGC, compressor ou limiter;
- margem para não atingir 0 dBFS;
- mesma passagem musical e, idealmente, referência de nível conhecido;
- loopback do SUN na mesma interface e mesmo ganho de entrada.

Com as cadeias controladas, separar ganho FM de saída, resposta MO/RO/placa, filtro analógico/acoplamento e diferenças de síntese. A calibração deve ficar no estágio de saída configurável, sem alterar amplitude interna nem temporização YM2413.

## 7. Referências

- Yamaha, *YM2413 Application Manual*, seção IV-2: MO/RO são source followers; requerem circuito externo/amplificação e o manual menciona LPF de aproximadamente 20 kHz para reduzir step noise. [SMS Power — manual Yamaha](https://www.smspower.org/maxim/Documents/YM2413ApplicationManual)
- [SMS Power — YM2413](https://www.smspower.org/Development/YM2413)
- [MAME Testers #5849 — balanço FM/PSG](https://mametesters.org/view.php?id=5849)
- Arquitetura SUN: `docs/audio/sms-complete-audio-architecture.md`, que exige calibrar ganho/resposta analógica separadamente e não ativar LPF genérico sem evidência.

## 8. Artefato A/B

Foi gerado fora do repositório um arquivo A/B para audição com RMS equivalente. Para evitar clipping, a referência física foi atenuada em **21,16 dB**; o trecho SUN permaneceu no nível original. Ordem: hardware primeiro, SUN depois. É normalização para comparação auditiva, não alteração no emulador.
