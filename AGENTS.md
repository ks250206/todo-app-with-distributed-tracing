# AGENTS.md

このファイルはリポジトリ全体に適用する開発ルールです。実装と利用方法の説明はルートの`README.md`を正とし、サブディレクトリへREADMEやAGENTSを増やさないでください。

## Project structure

- `backend/`: Rust、Axum、SQLx、SQLite
- `frontend/`: Vite Plus、React、Tailwind CSS
- `Caddyfile`: HTTPS、Frontend、API、管理UIのrouting
- `observability-Caddyfile`: Jaeger、Prometheus、Grafanaのadmin認可gateway
- `otel-collector-config.yaml`: trace、spanmetrics、Backend/Caddy log pipeline
- `alloy-config.alloy`: Frontend Faro eventの受信とLoki転送
- `loki-config.yaml`: log保存設定
- `grafana/provisioning/`: datasourceとDashboard provider
- `grafana/dashboards/`: version管理するGrafana Dashboard JSON
- `JustFile`: 統合起動、停止、状態確認

## Backend architecture

Backendは次の依存方向を維持してください。

```text
interfaces -> application -> domain
infrastructure ------------> domain
```

- Domain entity、value object、Repository interfaceは`backend/src/domain/`に置く。
- Application serviceはdomainの抽象にだけ依存する。
- SQLiteなどのRepository実装は`backend/src/infrastructure/`に置き、applicationへ注入する。
- HTTP request/response、Cookie、status codeは`backend/src/interfaces/`に閉じ込める。
- Domain invariantをHTTP handlerだけのvalidationにしない。constructorまたはvalue objectで必ず検証する。
- Repository errorは型付きのapplication errorへ変換し、既知の競合や認証エラーを`500`にしない。

## Security invariants

次の仕様を変更する場合は、対応するテストとREADMEも同時に更新してください。

- Todoの取得、更新、削除は必ず`user_id`をSQL条件に含める。他ユーザーのTodo IDは`404`として扱う。
- 最初のユーザーだけを`admin`にする。判定は`BEGIN IMMEDIATE`内で行い、DBの一意制約も維持する。
- Backendは`127.0.0.1:3000`だけで待ち受ける。LANへ直接公開しない。
- 公開CaddyがBackendでadmin権限を検証し、共有secretで内部gatewayへの直接アクセスを防ぐ。Jaeger、Prometheus、Grafana、Lokiのポートをホストへ直接公開しない。
- `/api/internal/*`を公開Caddy経由で公開しない。
- `/otel/*`は有効なセッションを検証してからCollectorへ転送する。
- `/faro/*`は有効なセッションを検証してからAlloyへ転送し、AlloyのFaro受信ポートはloopbackだけへbindする。
- パスワードはpepperを加えたArgon2idでハッシュ化し、平文やpepperをログへ出さない。
- Access TokenとRefresh TokenはDBへ平文保存せず、SHA-256ハッシュだけを保存する。
- 認証Cookieは`Secure; HttpOnly; SameSite=Lax; Path=/`を維持する。
- Refresh Tokenは使用時にAccess Tokenとともにローテーションする。
- Access Token期限切れだけではセッションを削除しない。Refresh Token期限切れ時だけ削除する。
- Login/Registerの送信元・アカウント単位rate limitと、未知メールに対するdummy Argon2id検証を維持する。
- `.env`、SQLite DB、token、password、pepperをcommitしない。

## Database changes

- 既存の`backend/app.db`を破壊しない後方互換なmigrationにする。
- schema変更は新規DBと既存DBの両方で繰り返し実行できるようにする。
- SQLite接続では外部キー制約を常に有効にする。
- 所有権、role、unique constraintに関わる変更にはRepository testを追加する。

## Frontend toolchain

Frontendのpackage managerはpnpmですが、操作にはVite Plusの`vp` commandを使用してください。

- 依存関係のinstall: `vp install`
- 依存関係の追加: `vp add <package>`または`vp add -D <package>`
- 開発サーバー: `vp dev`
- format、lint、型検査: `vp check`
- 自動修正: `vp check --fix`
- test: `vp test run`
- production build: `vp build`

`pnpm`、`npm`、`yarn`を直接実行しないでください。Vitest、Oxlint、Oxfmt、tsdownを個別にinstallしないでください。これらはVite Plus内蔵版を使用します。

- Vite設定は`vite-plus`からimportする。
- test utilityは`vite-plus/test`からimportする。`vitest`からimportしない。
- `node_modules/`、`dist/`、lockfileを手作業で編集しない。
- 変更後は`vp check`、`vp test run`、`vp build`を実行する。

## Frontend architecture

- API由来のserver state、cache、mutationはTanStack Queryで管理する。
- filterや表示modeなどのclient-only UI stateだけをZustandで管理する。
- server stateをZustandへ複製しない。
- Login/Registerは`/`、認証済み画面は`/dashboard/`とする。
- ログイン成功後は`/dashboard/`、ログアウト成功後は`/`へredirectする。
- Jaeger、Prometheus、Grafanaへのリンクは`role === "admin"`の場合だけ表示する。Frontendで隠すだけでなく、gatewayの認可を必ず維持する。
- API requestは`credentials: "include"`を使い、`401`時のRefreshは並行要求間で共有して一度だけ実行する。
- Refreshが`401`の場合はsession cacheを破棄し、`/`へ戻す。Refreshの`5xx`を認証失効として扱わない。
- mutation成功時は対象queryだけを更新または無効化し、無関係なcacheを全削除しない。

## OpenTelemetry

- BackendのHTTP、認証、Todo use caseのtraceを維持する。
- Frontendのfetch instrumentationと主要mutation spanを維持する。
- Browser traceは同一originの`/otel/v1/traces`へ送る。
- trace、span attribute、logへpassword、token、Cookie、pepper、個人情報を記録しない。
- Caddy access logはJSON encode、request/response headerとquery parameterの削除、client IP maskを維持し、Collectorのfilelog receiverで収集する。
- Backend logは構造化JSONとし、password、token、Cookie、pepper、メールアドレスを含めない。HTTP request logではtrace IDとspan IDの相関を維持する。
- Collectorのfilelog offsetを永続化し、再起動によるCaddy/Backend logの重複収集を避ける。
- Caddy/Backend logはCollectorからLokiへOTLPで送り、FrontendのFaro errorはAlloyからLokiへ送る。
- Faro SDKではconsole収集、user metadata、user agent、URLのquery/fragmentを送らない。初期renderを妨げないよう遅延loadを維持する。
- `/otel/v1/traces`自身をFrontendのfetch instrumentation対象から除外し、自己計装loopを防ぐ。
- `/faro/collect`自身もFrontend instrumentation対象から除外し、自己計装loopを防ぐ。
- Jaeger Monitor用のspanmetrics pipelineとPrometheus scrape設定を壊さない。
- Grafana datasource provisioningと、LokiからJaegerへ遷移するtrace derived fieldを壊さない。
- `Edge Tasks Overview`はfile provisioningを正とし、UI上の変更だけで管理しない。panel queryを変更した場合は実際のPrometheus/Loki labelと照合する。

## Integrated runtime

- 統合動作はリポジトリルートで`just start`を使用する。
- `just start`はFrontendをproduction buildし、BackendをCargo release profileでbuildして、Caddy、Jaeger、Prometheus、Grafana、Loki、Alloy、Collector、内部gatewayとともに起動する。
- Caddy変更後は両方の設定を検証する。

```sh
caddy validate --config Caddyfile
caddy validate --adapter caddyfile --config observability-Caddyfile
```

- 起動状態は`just status`、停止は`just stop`を使用する。
- ローカルHTTPSの確認では`curl -k`を使用してよい。本番相当のコードでTLS検証を無効化しない。

## Required validation

変更範囲に応じて最低限次を実行してください。

```sh
cd backend
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings

cd ../frontend
vp check
vp test run
vp build
```

認証、role、Caddy、Cookieを変更した場合は、未認証`401`、一般ユーザー`403`、admin`200`のintegration behaviorも確認してください。
