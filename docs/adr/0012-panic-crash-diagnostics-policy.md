# ADR 0012 — política de panic e crash diagnostics

- **Estado:** Aceita; implementação pendente da fatia DP0-F5.
- **Data:** 2026-11-20.
- **Escopo:** crate `sega-observability` e hosts desktop que autorizam a coleta.
- **Relacionadas:** ADR 0011, `docs/architecture/observability.md`, auditoria DP-0.

## Contexto

O runtime Rust precisa preservar o comportamento normal de panic para depuração e falha visível. Ao mesmo tempo, o produto não pode criar arquivos de crash nem coletar backtraces automaticamente antes de consentimento explícito.

O hook atual captura timestamp, payload e backtrace em `sega-emulator-crash.log` quando `DIAGNOSTICS_ENABLED` está ativo e, em seguida, sempre delega ao hook padrão previamente instalado. A saída padrão do panic pode continuar visível em stderr/debugger mesmo quando o arquivo privado não é escrito. Essas duas saídas têm finalidades e controles diferentes.

## Decisão

### 1. Preservar o hook padrão do Rust

- O hook padrão instalado antes do hook de diagnóstico deve sempre ser chamado exatamente uma vez.
- A coleta de diagnóstico não pode suprimir, substituir silenciosamente ou transformar um panic em recuperação.
- Não usar `catch_unwind` para esconder falhas de hardware/runtime.
- A revogação do consentimento desativa somente a captura privada; não altera o comportamento de falha do processo nem a saída padrão do panic.

### 2. Captura privada somente durante consentimento ativo

- Antes do opt-in: não instalar hook customizado, não criar diretório/arquivo de crash e não capturar backtrace.
- Após opt-in explícito: o hook customizado pode gravar um registro de crash local.
- Se o usuário revogar o consentimento antes do panic, nenhum registro privado é escrito.
- O consentimento é válido somente para a sessão e não é persistido.
- A captura deve consultar o estado de consentimento no momento do panic, não o estado que existia quando o hook foi instalado.

### 3. Minimização de dados

O registro privado padrão pode conter somente:
- timestamp UTC/Unix;
- identificador da sessão;
- nome-base do arquivo-fonte, linha e coluna, quando disponíveis;
- indicação de presença de payload textual, sem seu conteúdo.

Não gravar:
- texto arbitrário do payload do panic;
- caminhos absolutos ou diretórios pessoais;
- backtrace completo por padrão;
- ROM/BIOS, memória, SRAM, conteúdo de arquivo, hashes de ROM ou dados de entrada;
- dumps de memória ou minidumps.

A decisão de não gravar payload e backtrace por padrão é deliberada: ambos podem carregar dados sensíveis e o backtrace pode revelar caminhos de build. Uma futura exportação avançada exige finalidade explicada, ação separada do usuário e sanitização testada.

### 4. Falha segura e limites operacionais

- Falha ao abrir, escrever ou fazer flush do arquivo de crash não pode impedir a chamada do hook padrão.
- O hook não pode chamar logger, subscriber, worker assíncrono, GUI, locks de aplicação ou APIs do Scheduler.
- A escrita síncrona é permitida exclusivamente no caminho excepcional de panic; o caminho normal de hardware permanece sem I/O.
- O arquivo de crash deve ter limite e retenção finitos, aplicados antes de instalar o hook ou por mecanismo que não faça rotação complexa dentro do hook.
- Não enviar automaticamente crash reports ou logs para rede/terceiros.
- Não alterar o código de saída, a semântica de panic ou a ordem causal da máquina.

## Consequências

- O hook padrão continua disponível para debugger e stderr, inclusive quando a captura privada está desligada.
- A existência de um arquivo de crash passa a significar que o usuário havia autorizado diagnósticos no instante do panic.
- O crash log privado fica intencionalmente menos detalhado que um backtrace bruto; investigação profunda deve usar símbolos/build reproduzível ou uma exportação manual futura.
- A política evita que um panic cause coleta silenciosa de payloads arbitrários, caminhos absolutos ou backtraces potencialmente sensíveis.

## Plano de implementação e critérios de aceite — DP0-F5

1. Extrair um formatter puro para o registro sanitizado e testá-lo com payload contendo caminho, nome de ROM e texto sentinela; nenhum conteúdo sentinela pode aparecer na saída.
2. Testar que o hook customizado só é instalado após a ativação consentida da infraestrutura.
3. Testar em subprocesso que o hook padrão é chamado e que o processo continua terminando com comportamento de panic normal.
4. Testar em subprocesso a matriz: sem opt-in, opt-in ativo e opt-in revogado antes do panic.
5. Testar falhas de I/O sem impedir a delegação ao hook padrão.
6. Definir e testar retenção/limite finito do arquivo de crash sem executar rotação dentro do hook.
7. Executar os testes em Windows x64 Debug e Release; não alegar equivalência de saída stderr entre ambientes sem evidência.
8. Atualizar `docs/architecture/observability.md` e a auditoria DP-0 com os SHAs e resultados efetivamente observados.

## Fora de escopo

Esta ADR não decide a remoção do histórico de escritas do `SmsTemporalBus`, o limite do trace direto do core, o inventário completo de sinks nem os testes de fila/overhead de DP0-F6/F7. Também não autoriza alterar Scheduler, CPU, VDP, PSG, barramentos ou temporização emulada.
