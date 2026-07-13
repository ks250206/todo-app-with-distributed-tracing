# Edge Tasks

認証・認可、Todo CRUD、分散トレーシングを備えたフルスタックのサンプルアプリケーションです。

- Backend: Rust、Axum、SQLx、SQLite
- Frontend: Vite Plus、React、Tailwind CSS
- Server state: TanStack Query
- UI state: Zustand
- Authentication: Argon2id + pepper、Access Token、Refresh Token
- Observability: OpenTelemetry、Jaeger、Prometheus、Grafana、Loki、Alloy
- HTTPS / reverse proxy: Caddy

## 必要なもの

- [Nix](https://nixos.org/download/)（推奨。`flake.nix`で開発ツールを揃える）
- Podman（macOSではホストアプリとして別途インストール）
- または同等の手動インストール: Rust toolchain、[Vite Plus](https://viteplus.dev/)、just、Caddy、curl、OpenSSL、lsof

Vite Plusが`packageManager`の指定に従ってpnpmを使用します。Oxfmt、Oxlint、Vitestは個別にインストールせず、Vite Plus内蔵版を使用します。

## Nix セットアップ

```sh
nix develop
# direnv を使う場合
direnv allow
```

`nix develop`はRust（rustfmt/clippy含む）、just、Caddy、OpenSSL、curl、lsof、Vite Plus（`vp`）を提供します。LinuxではPodmanもshellに含めます。macOSのPodmanはホスト側のインストールを使います。

起動・停止などの実装は`scripts/`にあり、`JustFile`は薄いラッパーです。

## セットアップ

```sh
cp backend/.env.example backend/.env
```

`backend/.env`の`PASSWORD_PEPPER`を32文字以上のランダムな値へ変更してください。

```sh
openssl rand -base64 48
```

frontendの依存関係をインストールします。

```sh
cd frontend
vp install
cd ..
```

## 起動

```sh
just start
```

`just start`は次をまとめて実行します。

1. Jaeger、Prometheus、Grafana、Loki、Grafana Alloy、OpenTelemetry Collector、内部Caddyゲートウェイを起動
2. Backendをrelease buildで起動してSQLite migrationを適用
3. Frontendをproduction build
4. Caddyを起動またはreload

アクセス先は次のとおりです。

| サービス | URL | アクセス条件 |
|---|---|---|
| Login / Register | `https://localhost/` | 全ユーザー |
| Dashboard | `https://localhost/dashboard/` | ログイン済み |
| Jaeger UI | `https://localhost/jaeger/` | adminのみ |
| Prometheus UI | `https://localhost/prometheus/` | adminのみ |
| Grafana | `https://localhost/grafana/` | adminのみ |

CaddyはローカルCAでHTTPSを提供します。証明書が信頼されていない場合は次を実行してください。

```sh
caddy trust
```

停止と状態確認:

```sh
just status
just stop
```

Backendの構造化logは`.run/logs/backend.json`、Caddy自体のruntime logは`.run/caddy.log`へ出力されます。公開Caddyと管理UI gatewayのaccess logも`.run/logs/`へ構造化JSONで保存され、OpenTelemetry Collectorへ取り込まれます。コンテナのログは次のrecipeで確認できます。

```sh
just jaeger-logs
just otel-collector-logs
just prometheus-logs
just loki-logs
just grafana-logs
just alloy-logs
```

## ユーザーと権限

最初に登録されたユーザーだけが`admin`になります。2人目以降は`user`です。既存DBへrole migrationを適用する場合は、最小IDのユーザーがadminになります。

adminの決定はSQLiteの`BEGIN IMMEDIATE`トランザクション内で行い、部分一意indexによってadminが複数作成されることを防ぎます。

Backendは`127.0.0.1:3000`だけで待ち受け、LANへ直接公開しません。Jaeger、Prometheus、Grafana、Lokiのポートもホストへ直接公開されません。公開CaddyがBackendでadmin権限を検証し、起動時に生成される共有secretを持つ要求だけを内部Caddyゲートウェイが受け付けます。共有secretは`.run/`に権限を制限して保存され、`just stop`で削除されます。

- 未認証: `401 Unauthorized`
- 一般ユーザー: `403 Forbidden`
- admin: アクセス許可

FrontendでもJaeger、Prometheus、Grafanaへのリンクはadminにだけ表示します。

## 認証とセッション

パスワードはpepperを加えたうえでArgon2idによりハッシュ化します。パスワードは12文字以上必要です。

ログインまたは登録時に次のCookieを設定します。

- `access_token`: 有効期限15分
- `refresh_token`: 有効期限30日
- 属性: `Secure; HttpOnly; SameSite=Lax; Path=/`

トークン本体はDBへ保存せず、SHA-256ハッシュだけを保存します。Access Tokenが期限切れになっても、有効なRefresh Tokenを含むセッションは削除しません。Refresh時はAccess TokenとRefresh Tokenを両方ローテーションし、Refresh Token自体が期限切れの場合だけセッションを削除します。FrontendはAPIから`401`を受けた場合、Refreshを一度だけ実行して元の要求を再試行します。Refresh Tokenも無効な場合はsession cacheを破棄して`/`へ戻ります。

存在しないメールアドレスでのログインでもdummy Argon2id検証を実行し、ユーザーの有無による応答時間差を小さくします。LoginとRegisterには送信元IPおよびアカウント単位のメモリ内レート制限があり、超過時は`429 Too Many Requests`と`Retry-After: 60`を返します。

ログイン成功後は`/dashboard/`へ、ログアウト成功後は`/`へredirectします。

## Todo API

Todo APIは認証middlewareを必ず通り、RepositoryのSQLにも`user_id`条件を含めています。他ユーザーのTodoは一覧・取得・更新・削除できません。

| Method | Path | Body / 説明 |
|---|---|---|
| `POST` | `/api/auth/register` | `{ "email": "...", "password": "..." }` |
| `POST` | `/api/auth/login` | `{ "email": "...", "password": "..." }` |
| `POST` | `/api/auth/refresh` | CookieのRefresh Tokenをローテーション |
| `POST` | `/api/auth/logout` | セッションを破棄 |
| `GET` | `/api/me` | 現在の`id`、`email`、`role` |
| `GET` | `/api/todos` | 自分のTodo一覧 |
| `POST` | `/api/todos` | `{ "title": "..." }` |
| `GET` | `/api/todos/{id}` | 自分のTodoを取得 |
| `PATCH` | `/api/todos/{id}` | `{ "title": "...", "completed": true }` |
| `DELETE` | `/api/todos/{id}` | 自分のTodoを削除 |

他ユーザーが所有するTodo IDは`404 Not Found`として扱います。メールアドレスの重複は`409 Conflict`です。

## Frontend開発

```sh
cd frontend
vp dev
```

これはFrontend単体の開発サーバーです。HTTPS、Secure Cookie、Backend、管理UIを含む結合確認にはリポジトリルートの`just start`を使用してください。

品質チェックとテスト:

```sh
vp check
vp test run
vp build
```

`vp check`はOxfmt、Oxlint、TypeScriptの型検査をまとめて実行します。API由来の状態とmutationはTanStack Query、Todoフィルターなどの画面状態はZustandで管理します。

## Backend開発

```sh
cd backend
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

Backendはdomain、application、infrastructure、interfacesのレイヤーに分割しています。Repository interfaceはdomainに置き、applicationが抽象へ依存し、SQLite実装はinfrastructureから注入します。SQLite接続では外部キー制約を有効化し、Todoの部分更新は`COALESCE`を使った単一SQLで原子的に実行します。

## OpenTelemetry

BackendのHTTP request、認証、Todo操作にspanを作成します。Frontendではfetchを自動計装し、API呼び出しをBackendと同じ`METHOD /api/...`形式の`SERVER` spanとして記録します。ログイン、ログアウト、Todo mutationには追加の`INTERNAL` spanを付けます。

```text
Browser ──OTLP/HTTP──┐
                     ├─ OpenTelemetry Collector ─ Jaeger
Backend ──OTLP/gRPC──┘                         └─ spanmetrics ─ Prometheus

Caddy/Backend ──JSON log── Collector filelog ──OTLP──┐
Browser ──Faro Web SDK── Grafana Alloy ──────────────┼─ Loki ─ Grafana
```

Browserのtraceは同一originの`/otel/v1/traces`へ送信されます。公開Caddyは`/api/*`をOpenTelemetryでtraceし、W3C trace contextをBackendへ伝播します。そのためJaegerではFrontend API span → Caddy `caddy.api` span → Axum spanの経路を確認できます。`GET /`と`GET /dashboard/*`のHTML/SPA navigationも`caddy.frontend`としてtraceします。一方で`/assets/*`などの静的asset、`/otel/*`、`/faro/*`、管理UIはtraceしないため、assetごとのspanでJaegerを埋めません。Caddyは有効なログインセッションを確認してからCollectorのOTLP/HTTP endpointへ転送するため、未認証のtrace注入は`401`で拒否されます。JaegerのMonitor画面はCollectorのspanmetricsをPrometheusから取得します。

Caddy access logはrequest/response headerとquery parameterを削除し、client IPをマスクしてからJSON encodeします。Caddy tracingが付与する`traceID`と`spanID`もaccess logへ保存され、GrafanaからJaegerへ遷移できます。BackendもJSON logを出力し、HTTP spanの`trace_id`と`span_id`を相関情報として含めます。Collectorはそれぞれ`service.name=caddy`、`service.name=axum-crud`を付与したOTel LogRecordとして収集し、offsetを`.run/otelcol/`へ保存してLokiへ送信します。収集状態はPrometheusで次のmetricを確認できます。

- `otelcol_receiver_accepted_log_records_total`
- `otelcol_exporter_sent_log_records_total`
- `otelcol_fileconsumer_open_files_ratio`
- `otelcol_fileconsumer_reading_files_ratio`

FrontendはGrafana Faro Web SDKを初期描画後に遅延loadし、認証済みの同一origin endpoint `/faro/collect`からAlloyへbrowser errorを送信します。query parameter、user metadata、user agent、console出力は収集しません。Faro endpointはセッション認証、64KBのrequest body上限、送信rate limitを通すため、未認証のevent注入は拒否されます。ログイン済みの一般ユーザーも自端末のerror/performance eventを送れますが、admin専用のGrafanaログ閲覧と組み合わせる前提のローカル検証向け構成です。AlloyのFaro受信はホストへ`127.0.0.1:12347`だけ公開し、公開Caddy経由だけで転送します。既存のOTel browser traceとは役割を分け、Faroからtraceを二重送信しません。

Collectorのfilelog offsetは`.run/otelcol/logs`へ保存します。旧`.run/otelcol/caddy`がある場合、`just start`時に自動移行して重複収集を避けます。

GrafanaのExploreでは、provision済みのLoki datasourceから次のLogQLで確認できます。

```logql
{service_name="caddy"}
{service_name="axum-crud"}
{service_name="todo-frontend"}
```

GrafanaにはPrometheusとJaegerもprovision済みです。LokiのBackend logに含まれるtrace IDからJaegerを開けるderived fieldも設定しています。Lokiの保持期間はローカル検証向けに7日間で、Loki、Grafana、AlloyのdataはPodman named volumeへ保存されるため`just stop`後も維持されます。

Grafanaを開くと、provision済みの`Edge Tasks Overview` Dashboardがホームとして表示されます。API request rate、P95 latency、error数、観測基盤のhealth、サービス別log量、Collector/Alloyのlog throughput、最新logを確認できます。上部の`Service` filterで`caddy`、`axum-crud`、`todo-frontend`を切り替えられ、下部の個別パネルではCaddy access log、Backend log、Frontend Faro logをそれぞれ閲覧できます。Dashboard上部の`ログをテーブルで見る`から`Edge Tasks Log Table`へ移動すると、JSON fieldを列として表示できます。このDashboardはrepository内のJSONを正とするためGrafana UIから直接保存せず、`grafana/dashboards/`下のJSONを更新してください。

Monitorではサービス`todo-frontend`（Frontend）と`axum-crud`（Backend）を選べます。FrontendのAPI spanはfetch自動計装による`client` spanであり、Monitorではspan kindを`client`へ切り替えて確認します。業務上のmutation span（例: `auth.login`）はHTTP spanの親として維持されます。Todo IDは`/api/todos/{id}`へ正規化し、operationがIDごとに分裂しないようにしています。

## ディレクトリ

```text
.
├── backend/                    # Rust/Axum API
├── frontend/                   # Vite Plus/React UI
├── grafana/                    # datasourceとDashboard provisioning
├── scripts/                    # just から呼ぶ起動・停止スクリプト
├── alloy-config.alloy          # Frontend Faro event receiver
├── Caddyfile                   # HTTPS、API、Frontend routing
├── observability-Caddyfile     # 管理UIのadmin認可gateway
├── loki-config.yaml
├── otel-collector-config.yaml
├── prometheus.yml
├── ui-config.json
├── flake.nix                   # nix develop 用ツールチェーン
└── JustFile                    # scripts/ への薄いラッパー
```

## トラブルシューティング

### `curl: (7) Failed to connect to localhost port 443`

`just status`でCaddyとBackendを確認し、停止している場合は`just start`を再実行してください。

### Jaeger、Prometheus、Grafanaが`401`

ログインしていないか、Access Tokenの有効期限が切れています。アプリへ戻ってセッションをrefreshしてから再度アクセスしてください。

### Jaeger、Prometheus、Grafanaが`403`

ログイン中のユーザーにadmin権限がありません。管理UIへアクセスできるのは最初のユーザーだけです。

### 起動時に`PASSWORD_PEPPER`エラーになる

`backend/.env`が存在し、32文字以上の`PASSWORD_PEPPER`が設定されていることを確認してください。

### SQLite DBを空の状態へ戻したい

Backendの起動中に`backend/app.db`だけを削除すると、実行中のプロセスは削除前のDBを開いたまま動作します。必ず次のrecipeでBackendを停止してからDB本体とWALを削除し、再起動してください。

```sh
just reset-db
just start
```

`just reset-db`はユーザー、セッション、Todoをすべて削除します。

### LoginまたはRegisterが`429`になる

短時間に認証要求が集中したためrate limitが適用されています。`Retry-After`ヘッダーの秒数が経過してから再試行してください。
