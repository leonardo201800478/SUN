# Catálogo de firmware SMS com jogos incorporados

**Data da auditoria:** 2026-11-15
**Escopo:** inventário e identificação de imagens de firmware SMS fornecidas localmente pelo usuário.
**Status:** IDENTIDADES DOS ARQUIVOS VERIFICADAS / MAPEAMENTO E BOOT AINDA NÃO VALIDADOS.

## Objetivo

Documentar imagens de firmware do Sega Master System que incluem jogos na própria ROM de inicialização. Essas imagens não devem ser tratadas como cartuchos comuns nem como substitutas intercambiáveis das BIOS convencionais de 8 KiB.

O firmware pode executar uma rotina de inicialização, detectar os slots e transferir o controle ao cartucho quando presente. Quando não há cartucho, a execução pode continuar no conteúdo de jogo incorporado. O comportamento exato deve ser reproduzido pela execução do firmware e pelo mapeamento de hardware correspondente, não por uma condição artificial na GUI.

Este documento complementa [sms-bios-firmware-loading.md](sms-bios-firmware-loading.md), que registra as três BIOS convencionais de 8 KiB e o caminho de boot já existente no projeto.

## Inventário de imagens fornecidas

Os arquivos ZIP anexados pelo usuário foram extraídos localmente. Os hashes abaixo foram calculados sobre o conteúdo binário do arquivo `.sms`, não sobre o ZIP. Os binários não foram adicionados ao repositório.

| Identificador proposto | Nome de origem | Tamanho | CRC32 | SHA-1 | SHA-256 | MD5 |
|---|---|---:|---|---|---|---|
| `sms2-sonic-europe` | Sonic The Hedgehog (Europe) | 262.144 bytes (256 KiB) | `81C3476B` | `6aca0e3dffe461ba1cb11a86cd4caf5b97e1b8df` | `c51ec80b7a609c5ee3d203ceee2b602ae4dfb4479caf15dc1d030c40c0364e93` | `4187d96beaf36385e681a3cf3bd1663d` |
| `sms-hang-on-v34` | Hang On (USA, Europe) (v3.4) | 131.072 bytes (128 KiB) | `8EDF7AC6` | `51fd6d7990f62cd9d18c9ecfc62ed7936169107e` | `1a73bc6e1a9973b98432774a1480b3ac1259027a97590d5e67d52745159abbbc` | `5cd8f62cd8786af0226e6d2248279338` |
| `sms-hang-on-safari-hunt-v24` | Hang On & Safari Hunt (USA, Europe, Brazil) (En) (v2.4) | 131.072 bytes (128 KiB) | `91E93385` | `9e179392cd416af14024d8f31c981d9ee9a64517` | `e9eb04ec80ded8658f765c4eca75cdbe23cbc7d8fed73f9c3db7a868dbf649ff` | `02cbb2e348945c9ac41e37502a58ca76` |
| `sms-missile-defense-3d-v44` | Missile Defense 3-D (USA, Europe) (v4.4) | 131.072 bytes (128 KiB) | `E79BB689` | `aa92ae576ca670b00855e278378d89e9f85e0351` | `13c0639cc037167f53e6151dbdea013758c3d09a76b064a894bf15c39c0a3897` | `08b81aa6be18b92daef1b875deecf824` |

### Firmware Samsung Gam*Boy II — Alex Kidd

Imagem identificada anteriormente a partir de arquivo local fornecido pelo usuário:

| Identificador proposto | Nome de origem | Tamanho | CRC32 | SHA-1 | SHA-256 | MD5 |
|---|---|---:|---|---|---|---|
| `samsung-gamboy-ii-korea-alex-kidd` | Samsung Gam-Boy 2 BIOS (Alex Kidd in Miracle World) (K) | 131.072 bytes (128 KiB) | `9C5BAD91` | `2feafd8f1c40fdf1bd5668f8c5c02e5560945b17` | `57abf3c00c82a8e042ebeb2bfe412adac9ba005e9009845236390de3ee645248` | `e49e63328c78fe3d24ae32bf903583e0` |

A imagem coreana deve ter identidade e perfil próprios. Ela não é alias da BIOS japonesa v2.1 nem de uma BIOS SMS genérica de 8 KiB.

## Referência das BIOS convencionais

As três imagens convencionais de 8 KiB permanecem documentadas em [sms-bios-firmware-loading.md](sms-bios-firmware-loading.md):

- Japan v2.1 — SHA-256 `67846e26764bd862f19179294347f7353a4166b62ac4198a5ec32933b7da486e`.
- USA/Europe v1.3 — SHA-256 `477617917a12a30f9f43844909dc2de6e6a617430f5c9a36306c86414a670d50`.
- Europe v2.0 — SHA-256 `1eb16fc06ceaf05651d7cfd69d1713e6ddccc4349a511f6fd17e037d71f9d85b`.

O catálogo passa, portanto, a registrar pelo menos oito imagens: três BIOS convencionais e cinco imagens com jogos incorporados, incluindo a variante coreana.

## Comportamento de inicialização esperado

O modelo funcional a validar para firmware com jogo incorporado é:

1. Power-on/reset executa o firmware.
2. O firmware realiza sua inicialização e verifica a presença de cartucho/slot conforme o hardware da variante.
3. Se houver cartucho, o firmware transfere o controle ao cartucho.
4. Se não houver cartucho, o firmware continua sua rotina e inicia o jogo incorporado.

Esse fluxo é uma descrição funcional de alto nível. Não autoriza assumir que todas as imagens usam o mesmo mapa de memória, a mesma revisão de ASIC ou a mesma sequência de hand-off.

A documentação SMS Power sobre BIOS e slots descreve para a BIOS Samsung Gam*Boy II a verificação dos slots e o caminho para o jogo incorporado Alex Kidd. As imagens Hang On, Hang On & Safari Hunt e Missile Defense 3-D também são listadas como BIOS/firmwares com jogos internos e requerem tratamento de slot 2 na descrição técnica consultada. A imagem Sonic tem tamanho distinto (256 KiB); seu mecanismo de mapeamento precisa ser auditado separadamente.

## Regras arquiteturais para implementação

- Identificar firmware por SHA-256; CRC32, SHA-1 e MD5 ficam como metadados auxiliares para diagnóstico/interoperabilidade.
- Validar o tamanho esperado antes de ler integralmente o arquivo.
- Não depender de nome, extensão ou diretório para reconhecer conteúdo.
- Não limitar o scanner SMS a 8 KiB: ele precisa reconhecer os tamanhos de 128 KiB e 256 KiB catalogados.
- Não encaminhar imagens maiores ao loader convencional de BIOS de 8 KiB.
- Modelar tipo de firmware, variante de console, região, jogo incorporado, tamanho, hash e estratégia de mapeamento como campos distintos.
- A presença de cartucho deve ser observada pelo firmware/hardware emulado; a GUI não deve decidir qual jogo executar.
- Preservar a ROM e o mapper do cartucho, a sequência real de inicialização e o estado de CPU/RAM/registradores no hand-off.
- Manter separadas região de lançamento, território da BIOS, padrão de vídeo e clock. Não inferir PAL/NTSC a partir do país do nome do arquivo.
- A imagem Sonic Europe de 256 KiB não deve ser presumida equivalente às variantes Tec Toy brasileiras apenas por conter Sonic.
- Nenhum binário comercial/proprietário deve ser adicionado ao Git, CI ou artefatos permanentes.

## Pendências técnicas

### Identificação e catálogo
- [ ] Introduzir tipo de firmware distinto de `SmsBios8KiB`, capaz de representar firmware com jogo incorporado.
- [ ] Adicionar as cinco identidades deste documento ao catálogo, usando SHA-256 como chave primária.
- [ ] Testar nome arbitrário, extensão arbitrária, arquivo truncado, tamanho incorreto, duplicata, arquivo ilegível e conteúdo desconhecido.
- [ ] Confirmar os hashes em uma segunda execução independente e manter fixtures sem bytes proprietários.

### Hardware e boot
- [ ] Auditar o mapa de memória e a seleção de slots para cada família de firmware.
- [ ] Determinar como cada firmware expõe a ROM incorporada e como o cartucho externo obtém prioridade.
- [ ] Auditar separadamente a imagem Sonic Europe de 256 KiB.
- [ ] Implementar o mecanismo de slot 2 apenas onde a documentação/código de referência sustentar esse comportamento.
- [ ] Testar cartucho presente, slot vazio, reset, troca de cartucho e hand-off sem zerar artificialmente o estado da CPU.
- [ ] Validar que o boot direto sem BIOS convencional continua funcional.
- [ ] Não declarar boot real validado sem execução da imagem correspondente no runtime.

### Interface e integração
- [ ] Exibir firmware identificado, modelo/variante, jogo incorporado, tamanho e hash abreviado.
- [ ] Distinguir firmware detectado, selecionado, carregado e efetivamente executado.
- [ ] Preservar a GUI como camada de configuração/apresentação; nenhum controle de ciclos ou hardware deve migrar para Slint.
- [ ] Manter leitura de arquivos fora do caminho crítico do Scheduler.

### Validação
- [ ] Executar fmt, Clippy, testes direcionados e regressões SMS no Linux/CI.
- [ ] Verificar workflows no HEAD exato da PR.
- [ ] Testes Windows e validação visual/manual permanecem adiados nesta etapa; não registrar como PASS.
- [ ] Não afirmar equivalência com hardware físico sem evidência independente.

## Fontes técnicas e limites de evidência

- SMS Power — BIOSes: https://www.smspower.org/Development/BIOSes
- SMS Power — Port $3E: https://www.smspower.org/Development/Port3E
- SMS Power — ROM Header: https://www.smspower.org/Development/ROMHeader
- RetroBIOS — catálogo Sega: https://abdess.github.io/retrobios/systems/sega/
- MAME — Samsung Gam*Boy II com Alex Kidd: https://github.com/mamedev/mame

Os hashes e tamanhos desta auditoria foram calculados a partir dos arquivos locais anexados pelo usuário. A documentação externa sustenta a classificação geral e alguns detalhes de slot/boot, mas não substitui a auditoria do código do core nem a validação de cada dump no runtime. Este documento não afirma que qualquer uma das cinco imagens com jogo incorporado já esteja funcionalmente suportada pelo emulador.
