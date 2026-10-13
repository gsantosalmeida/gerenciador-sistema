# Gerenciador de Sistema

Um aplicativo desktop simples para organizar a pasta Downloads e limpar arquivos temporários no Windows. A interface foi pensada para deixar cada ação visível e permitir uma simulação antes de alterar arquivos.

> **Status:** em desenvolvimento. Revise os resultados da simulação antes de executar uma limpeza real.

## O que ele faz

- **Organiza Downloads:** move os arquivos que estão diretamente na pasta escolhida para subpastas por tipo, como PDFs, imagens, vídeos e arquivos compactados. A organização não depende da idade do arquivo.
- **Escolhe pastas em diálogo nativo:** use o seletor de pastas do Windows para definir Downloads e o seletor de arquivo para escolher onde salvar o registro.
- **Ícones personalizados:** o aplicativo e as ações usam os ícones do projeto, gerados nos formatos necessários para o instalador Windows.
- **Arquiva instaladores:** arquivos como `.exe`, `.msi`, `.msix` e `.appx` vão para `Instaladores arquivados`. O aplicativo não apaga instaladores automaticamente.
- **Limpa temporários por idade:** remove arquivos mais antigos que o limite escolhido nas pastas TEMP do Windows. Mantém os arquivos recentes e as pastas.
- **Limpa todo o conteúdo TEMP:** tenta remover arquivos e subpastas, inclusive itens recentes. Mantém as pastas TEMP em si; arquivos em uso ou sem acesso são ignorados.
- **Prefetch (opcional):** tenta remover arquivos `.pf` e `.db` diretamente da pasta Prefetch do Windows. Essa opção fica desativada por padrão; o Windows recria esses dados, e programas podem iniciar mais devagar até isso acontecer. Subpastas são preservadas.
- **Simula antes de executar:** a simulação vem ativada por padrão e não altera arquivos. Ao desativá-la, o aplicativo pede confirmação antes de executar as ações.
- **Mostra o resultado:** registra itens movidos, apagados, ignorados e erros, além de estimar o espaço liberado. É possível salvar o registro em um arquivo.

## Requisitos

### Para usar o aplicativo

O instalador será criado para Windows. Quem usar a versão empacotada não precisa instalar Node.js ou Rust.

### Para desenvolver

- Windows 10 ou posterior com Microsoft Edge WebView2 Runtime.
- Node.js LTS e npm.
- Rust com toolchain `stable-msvc`.
- Microsoft C++ Build Tools com a carga de trabalho **Desenvolvimento para desktop com C++**.

## Rodar em modo de desenvolvimento

1. Instale os requisitos acima.
2. Clone este repositório e entre na pasta do projeto:

   ```powershell
   git clone https://github.com/gsantosalmeida/gerenciador-sistema.git
   cd gerenciador-sistema
   ```

3. Instale as dependências da interface:

   ```powershell
   npm install
   ```

4. Inicie o aplicativo:

   ```powershell
   npm run tauri dev
   ```

## Gerar instalador do Windows

Na pasta do projeto, execute:

```powershell
npm run tauri build
```

Os pacotes compilados ficam dentro de `src-tauri/target/release/bundle/`, separados por formato de instalador.

## Segurança e privacidade

- A análise em modo de simulação não move nem apaga arquivos.
- A organização só considera arquivos diretamente na raiz da pasta Downloads escolhida; arquivos dentro de subpastas não são reorganizados.
- Instaladores são arquivados e preservados.
- A limpeza completa afeta o conteúdo das pastas temporárias reconhecidas pelo Windows. Feche os programas antes de usá-la.
- O aplicativo não envia os caminhos ou nomes dos arquivos para um serviço externo. O registro é gravado localmente apenas se você informar um caminho.
- Arquivos bloqueados, links e itens sem permissão podem ser ignorados; consulte o registro para ver o resultado.

## Estrutura do projeto

```text
src/                 Interface em TypeScript e CSS
src-tauri/src/       Comandos Rust e operações de arquivos
src-tauri/           Configuração e empacotamento do Tauri
```

## Contribuições

Sugestões e relatos de problemas são bem-vindos. Ao abrir uma issue, descreva o que esperava que acontecesse e o que ocorreu; evite anexar caminhos pessoais ou nomes de arquivos privados.

## Licença

Uma licença de código aberto ainda não foi definida. Até que uma licença seja adicionada ao repositório, não presuma permissão para redistribuir ou modificar o projeto.
