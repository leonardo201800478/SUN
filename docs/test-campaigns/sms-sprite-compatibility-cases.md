# SMS VDP — campanha de compatibilidade de sprites

**Status:** campanha exploratória em andamento  
**Objetivo:** registrar ROMs, cenas, resultados observados e evidências comparativas antes de alterar a implementação do VDP.  
**Hardware de referência do desktop:** perfil SMS1 / ASIC 315-5124, salvo indicação explícita em cada caso.  
**Regra:** resultados relatados pelo usuário são registrados como observações; não constituem, isoladamente, prova da causa técnica.

## Convenções

- **REPRODUZIDO PELO USUÁRIO:** comportamento observado no build local informado pelo usuário.
- **REFERÊNCIA ARES:** imagem ou comportamento usado como comparação externa.
- **IDENTIDADE PENDENTE:** ROM ainda sem CRC32/SHA-1 confirmado.
- **CAUSA NÃO DETERMINADA:** não atribuir o defeito a SAT, bitplanes, prioridade ou temporização sem trace/teste que discrimine a hipótese.
- Screenshots enviados na conversa são evidência visual de referência, mas não estão versionados neste documento. Quando os arquivos forem anexados ao repositório, registrar caminho e hash.

## Casos registrados

### SMS-SPR-001 — SMS 3-D Demo (World) (Aftermarket) (Unl)

| Campo | Registro |
|---|---|
| Sistema | Sega Master System |
| ROM | `SMS 3-D Demo (World) (Aftermarket) (Unl).sms` |
| CRC32 / SHA-1 | PENDENTE — arquivo ROM ainda não fornecido para identificação |
| Perfil do emulador | SMS1 / 315-5124 (perfil ativo do desktop no teste relatado) |
| Referência | Ares, mesma cena visual informada pelo usuário |
| Resultado no Ares | Objetos circulares verdes, sombras e elementos da cena renderizados de forma coerente |
| Resultado no Sega Emulator | Fundo e faixas de cores aparecem; objetos 3D/sprites aparecem fragmentados, deslocados ou substituídos por blocos |
| Classificação | REPRODUZIDO PELO USUÁRIO; IDENTIDADE PENDENTE |
| Causa | NÃO DETERMINADA |

**Evidência visual:** comparação lado a lado enviada pelo usuário em 14/11/2026. A janela à esquerda é o Sega Emulator; à direita, Ares. A imagem demonstra divergência visual na mesma demonstração, mas não isola por si só o mecanismo responsável.

**Perguntas a responder:**
1. A ROM usa exclusivamente sprites Mode 4 ou também altera registradores/modos durante a cena?
2. Os atributos SAT e os índices de padrão selecionados coincidem com a referência?
3. Os bytes dos padrões e bitplanes são lidos do endereço correto?
4. Há divergência em limite de oito sprites por linha, zoom, deslocamento X/Y, clipping ou prioridade?
5. O defeito persiste com execução determinística e após reset/reload?

**Próxima ação:** obter o arquivo exato, calcular CRC32/SHA-1 e registrar região/configuração, cena reproduzível e screenshot sem redimensionamento. Não alterar o VDP com base apenas neste caso.

### SMS-SPR-002 — Cosmic Spacehead (Europe) (En,Fr,De,Es)

| Campo | Registro |
|---|---|
| Sistema | Sega Master System |
| ROM | `Cosmic Spacehead (Europe) (En,Fr,De,Es).sms` |
| Tamanho | 262.144 bytes |
| CRC32 | `29822980` |
| SHA-1 | `f46f716dd34a1a5013a2d8a59769f6ef7536a567` |
| Mapper | Codemasters |
| Resultado relatado | Jogo inicia e áudio funciona; sprites aparecem totalmente fora de posição |
| Classificação | REPRODUZIDO PELO USUÁRIO |
| Causa | NÃO DETERMINADA; compatibilidade de modo/perfil de vídeo é hipótese a investigar |

**Nota de compatibilidade:** evidência de catálogo previamente consultada indica requisito de modo extra-height e compatibilidade associada a SMS2 para esta ROM. Isso deve ser verificado contra a configuração efetiva e a referência antes de classificar o sintoma como falha genérica de sprites SMS1.

### SMS-SPR-003 — The Smurfs Travel the World (Brazil)

| Campo | Registro |
|---|---|
| Sistema | Sega Master System |
| ROM | `Smurfs Travel the World, The (Brazil).sms` |
| Tamanho | 262.144 bytes |
| CRC32 | `E14C8A1C` |
| SHA-1 | `15b882ad3263cdc1e78c732d96f9bb18e25f4743` |
| Resultado relatado | Áudio funciona; sprites aparecem fora de posição |
| Classificação | REPRODUZIDO PELO USUÁRIO |
| Identidade comercial/dump | Não confirmada por catálogo conhecido na investigação inicial |
| Causa | NÃO DETERMINADA |

### SMS-BOOT-001 — Andre Agassi Tennis (Europe, Brazil) (En)

| Campo | Registro |
|---|---|
| Sistema | Sega Master System |
| ROM | `Andre Agassi Tennis (Europe, Brazil) (En).sms` |
| Tamanho | 262.144 bytes |
| CRC32 | `F499034D` |
| SHA-1 | `81dbacad4739b98281c750d9af21606275398fc8` |
| Resultado relatado | Tela totalmente negra e zunido contínuo; nenhuma imagem do jogo |
| Snapshot observado | PC variou entre `00BD`, `008E` e `0089` em capturas distintas; SP `DFF7` |
| Classificação | REPRODUZIDO PELO USUÁRIO |
| Causa | NÃO DETERMINADA |

Este caso é registrado como falha de boot/vídeo, não como caso isolado de sprites. O movimento do PC entre snapshots não demonstra, por si só, progresso correto do boot.

## Protocolo para novos casos

Para cada ROM nova, registrar:

1. Nome exato do arquivo e tamanho em bytes.
2. CRC32 e SHA-1.
3. Região e perfil de hardware selecionados (SMS1/315-5124, SMS2 ou outro).
4. Versão/commit do Sega Emulator e versão do Ares usada na comparação.
5. Cena, sequência de entrada e tempo desde o reset.
6. Resultado de vídeo e áudio em cada emulador.
7. Screenshot da mesma cena, sem filtros ou escalas diferentes quando possível.
8. Snapshot/trace, se disponível, e número de eventos descartados.
9. Estado da reprodução: reproduzido, não reproduzido ou inconclusivo.

## Matriz de investigação — não são conclusões

| Área | Verificação |
|---|---|
| SAT | Endereço base, leitura de Y, X, tile/pattern, atributos e ordem |
| Padrões | Endereçamento, seleção de pattern, bitplanes e leitura de VRAM |
| Geometria | 8×8/8×16, zoom, clipping e coordenadas negativas |
| Composição | Cor transparente, prioridade, sobreposição e ordem |
| Limites | Oito sprites por scanline, overflow e índice reportado |
| Colisão | Interseção de pixels opacos e instante de publicação do status |
| Temporização | Fetch de SAT/padrões, escritas intralinha e boundaries por dot/T-state |
| Perfil | Diferenças SMS1 315-5124 versus SMS2 e modos de vídeo especiais |

## Política de implementação

- Não modificar SAT, sprite renderer ou temporização para corrigir um único título sem teste que identifique o comportamento.
- Separar defeitos de semântica de sprite de defeitos de pipeline temporal.
- Criar testes sintéticos mínimos para cada comportamento isolado e manter os casos reais como regressão.
- Preservar Scheduler, determinismo e equivalência entre execução contínua e em blocos.
- Atualizar este documento incrementalmente à medida que novos testes forem recebidos.
