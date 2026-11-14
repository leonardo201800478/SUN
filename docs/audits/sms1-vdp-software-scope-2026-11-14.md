# SMS1 VDP — escopo de compatibilidade por software

**Data:** 14/11/2026  
**Baseline de trabalho:** branch `feat/sms1-left-edge-crop-overscan-20261114`  
**Objetivo:** priorizar compatibilidade funcional de jogos SMS1 sem depender de medições físicas ou de uma pistola óptica real.

## Decisões de escopo

- Light Phaser físico, sensores ópticos, captura elétrica TH e temporização física do H-counter não são requisitos desta etapa.
- A entrada de mira será software: mira branca desenhada na camada de apresentação e posição fornecida por mouse, analógico ou giroscópio.
- A mira não pode alterar framebuffer canônico, VDP, Scheduler ou estado temporal da máquina.
- Medições físicas não são gates para encerrar as lacunas funcionais que podem ser verificadas por testes automatizados e ROMs de diagnóstico executadas no próprio emulador.

## Implementações adicionadas nesta etapa

### Crop de borda esquerda

`SmsOverscanMode` oferece:

- `Off`: frame canônico 256×192.
- `CropLeft8`: cópia de apresentação 248×192, removendo os primeiros oito pixels de cada linha.

O método `SmsVdp::framebuffer_with_overscan` não altera o framebuffer original, registradores, raster position, relógio ou Scheduler. É um crop de apresentação, não uma alegação de que o VDP renderiza pixels de overscan fora da janela ativa.

O `SmsTemporalWorker::set_overscan_mode` aplica o modo aos frames publicados no `FrameMailbox`. O padrão continua `Off`. A persistência e o controle visual dessa preferência ficam para a próxima etapa de integração da GUI.

### Limite de sprites

A opção já existia e estava conectada ao worker e à preferência persistida da interface:

- `HardwareLimit`: limite de oito sprites por linha.
- `UnlockedCompatibility`: permite composição de até 64 entradas na linha para compatibilidade experimental.

O modo desbloqueado não altera o clock do VDP nem deve apagar o status de overflow real. A UI já oferece a alternância e persiste `sprite_limit_enabled`.

### Save States

O runtime agora oferece `SmsTemporalWorker::save_state()` e `load_state(state)`:

- Captura em boundary seguro do worker.
- Snapshot contém CPU, estado de continuação Z80, Scheduler, bus, mapper/memória, VDP, PSG, controladores e eventos pendentes.
- Restore exige worker pausado e usa `SmsTemporalMachine::restore_state`, que valida versões e invariantes antes de substituir a máquina.
- Após restore, telemetria de frame/tempo e epoch do renderer de áudio são sincronizados.

**Limite atual:** esta API captura/restaura snapshots em memória. A codificação persistente em arquivo, slots de save, identidade da ROM/checksum e escrita atômica ainda precisam ser implementadas. O armazenamento SRAM `.srm` existente não é Save State.

## Próximas lacunas de software

1. Executar e registrar a ROM SmsVdpTest dentro do runtime para cobrir os eventos já implementados.
2. Expandir testes de chunking, clone/restore e continuidade para CRAM/VRAM, overflow, colisão e fronteiras de frame.
3. Evoluir o modelo de fetch-ahead de tiles, bitplanes e padrões de sprites onde a evidência de ROMs de teste justificar.
4. Fechar a persistência binária de Save States e a compatibilidade/versionamento de arquivos.
5. Ligar as opções de vídeo à GUI Slint: crop, proporção, limite de sprites, mira, visibilidade/escala da mira e origem de entrada.
6. A mira deve transformar coordenadas de host para coordenadas lógicas após letterbox, aspect ratio e crop; nunca pode ser rasterizada no framebuffer do VDP.

## Critério de conclusão

A meta desta trilha é compatibilidade funcional ampla com jogos SMS1, preservando regressões SMS2/SG-1000 e determinismo. Não se exige pistola física, captura elétrica TH ou medição em placa real. A liberação continua condicionada à cobertura de software e aos testes automatizados no SHA candidato.
