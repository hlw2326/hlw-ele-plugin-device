# @hlw-ele/plugin-device

Windows 10+ x64 / Node.js 20.19+ 的原生设备采集包，使用 Rust、napi-rs 和 Windows API，不依赖 Electron，不运行 PowerShell、WMI 或命令行，不包含 IPC、网络请求和持久化。

```ts
import { device, type DeviceInfo } from '@hlw-ele/plugin-device'

const info: DeviceInfo = device.info()
```

## 在 hlw 中使用

包本身保持独立（`@hlw-ele/plugin-device` 的 `.` 入口不依赖 Electron），另外提供三个 hlw 装配入口：

```ts
// 宿主 main
import { initDevice } from '@hlw-ele/plugin-device/main'
initDevice()

// 宿主 preload
import { init } from '@hlw-ele/core/preload'
import { device } from '@hlw-ele/plugin-device/preload'
init({ device })

// 渲染层 / 子应用
const info = hlw.device.info()
```

类型通过 `declare module '@hlw-ele/types'` 合并，消费端加一行即可：

```ts
import '@hlw-ele/plugin-device/types'
```

IPC 频道固定为 `hlw:device:info`；原生采集只在 main 进程执行。

CommonJS 使用 `const { device } = require('@hlw-ele/plugin-device')`。唯一公开方法是 `device.info(): DeviceInfo`，同步返回、无参数，不增加 refresh/clear 方法或刷新选项。JS 入口延迟加载 binding，首次调用执行真实采集并缓存完整快照，后续直接复用；每次返回 `structuredClone` 独立副本，调用方修改不会污染缓存或其它调用方。采集或加载失败直接抛错，且不写入缓存，允许下次调用重试。Rust binding 不缓存，直接调用 binding 仍会执行真实采集。公开类型只有 `DeviceInfo`，没有 `Info` 别名、嵌套采集类型或 `issues`。

缓存仅存在于当前进程内的当前模块实例，不落盘、不跨进程共享；ESM/CJS 入口各有自己的模块缓存。**所有字段均为首次成功采集的快照，而非实时状态**，包括 `uptime`、`freeMemory`、`ip`、`mac`、屏幕尺寸/缩放及电池状态。用户需要重启应用才能刷新快照。

## 字段语义

返回值仅包含以下 25 个扁平字段；`?` 表示可省略，所有容量单位均为字节。

| 字段 | 类型 | 语义 |
| --- | --- | --- |
| did | string | 硬件标识：SMBIOS Type 1 原生 UUID 按现有 `decode_uuid` 处理版本字节序，规范化为小写带连字符字符串，其 UTF-8 字节的 SHA-256，输出小写 64hex |
| uuid | string | 系统安装标识：现有 `system::read_identity()` 读取的 64 位注册表视图 MachineGuid 字符串 UTF-8 字节的 SHA-256，输出小写 64hex |
| manufacturer? | string | SMBIOS Type 1 制造商 |
| model? | string | SMBIOS Type 1 产品型号 |
| os | string | Windows 平台标识，固定为 `win` |
| osName | string | 注册表 ProductName；客户端 build >= 22000 时修正遗留的 Windows 10 名称为 Windows 11，保留版本后缀，不改写 Server 名称 |
| release | string | `RtlGetVersion` 实际内核 `major.minor.build`，不是营销版本或 DisplayVersion |
| kernel | string | Windows 内核名称，固定为 `Windows_NT` |
| host | string | Windows DNS 主机名 |
| arch | string | 原生系统架构：`x64` / `arm64` / `ia32`，非进程架构 |
| uptime | number | Windows 开机后经过的秒数 |
| cpu | string | 注册表 ProcessorNameString 处理器型号 |
| cores? | number | 覆盖全部处理器组的活动逻辑核心数，非物理核心数 |
| gpu | string | DXGI 硬件显卡名称，多个以 `; ` 拼接，排除软件适配器，保留集显 |
| memory | number | 物理内存总量 |
| freeMemory? | number | Windows 可用物理内存，包含可回收页，不仅是完全空闲页 |
| disk | number | Windows 系统卷所在物理磁盘的完整容量，含其它分区与未分配空间，不是系统卷容量或所有磁盘总和 |
| mac | string | 所选网卡物理地址，小写冒号分隔；无物理地址时为空字符串 |
| ip? | string | 同一网卡的首个 IPv4，否则首个 IPv6，保留 IPv6 scope ID |
| netName | string | 同一网卡 FriendlyName |
| netType? | string | 同一网卡原生 IfType：`Ethernet` / `Wi-Fi`，其它动态表示为 `ifType:<数字>` |
| screenWidth | number | 主显示器当前模式物理像素宽度 |
| screenHeight | number | 主显示器当前模式物理像素高度 |
| screenScale? | number | 主显示器 Windows 有效缩放百分比 / 100，即有效 DPI / 96，例如 125% 对应 1.25 |
| battery? | boolean | Windows 电池存在状态；无电池为 false，状态未知则省略 |

四字段仅使用 `host`、`netName`、`screenScale`、`battery`，不导出旧名称 `hostname`、`netInterface`、`scaleFactor`、`hasBattery`，也不提供映射别名。`battery` 保持可选 boolean，是用户明确批准的布尔字段 `is` 前缀例外；其余字段及 `did` / `uuid` 计算不变。

### 原生读取与错误策略

- `disk`：`GetWindowsDirectoryW` → `GetVolumePathNameW` → 系统卷 `IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS` → `PhysicalDriveN` 的 `IOCTL_DISK_GET_DRIVE_GEOMETRY_EX.DiskSize`。使用零访问权限打开句柄，避免不必要的管理员权限请求。若系统卷无盘符、跨多个物理盘、驱动不支持查询或访问被拒绝，Promise 拒绝，不降级为卷容量。
- 屏幕：`MonitorFromPoint` / `GetMonitorInfoW` 定位主屏，`EnumDisplaySettingsExW` 获取不经 DPI 虚拟化的物理像素，`GetScaleFactorForMonitor` 读取真实 Windows 缩放。不会调用 `SetProcessDpiAwareness`、修改宿主进程或线程 DPI 状态；不使用会受进程 DPI awareness 影响的逻辑桌面尺寸。无可读主屏（如无显示输出的会话）时拒绝。
- 网卡：先从活动、非 loopback 接口中优先选择带有效 MAC 的网卡，排除空、全零和全 FF 的物理地址；在候选中按 IPv4/IPv6 默认路由的 route metric + interface metric 最小值选择，同分或无默认路由时按接口索引确定。仅当所有活动接口均无有效 MAC 时才选择无 MAC 的接口。VPN/TUN 的无 MAC 默认接口不会遮蔽带有效 MAC 的活动网卡；返回的 IP 因此可能是该网卡地址，而非隧道地址。不使用固定虚拟网卡过滤表，MAC、IP、名称、类型始终来自同一接口。只有断开的接口时视为无可选接口。
- `did` 和 `uuid` 均为必填字符串，不提供 `deviceId` 名称或原始 SMBIOS UUID 额外字段。SMBIOS Type 1 缺失、UUID 字段缺失、全零或全 FF 时拒绝整个 Promise，不回退到 MachineGuid 或其它标识。
- 必填数据采集失败拒绝整个 Promise；不输出部分成功对象，也不返回错误集合。可选数据真实缺失时 Rust `None` 在 JS 中表现为省略属性，而不是 `null` 或伪造默认值：制造商或型号字段不存在、没有单播 IP、没有可选接口、电池状态为 255。可选字段涉及的 API 若报错或数据损坏，同样拒绝，不静默吞掉系统错误。成功的核心数、可用内存和缩放查询都会返回实际值。
- 真实硬件显卡集合为空时 `gpu=''`；无活动非 loopback 网卡时 `netName=''`、`mac=''`，省略 `ip` 和 `netType`。这些是实际空集合，不是错误回退。

`did` 固件 UUID 哈希与 `uuid` MachineGuid 哈希可能随重装、克隆、虚拟机和固件设置改变或重复，不是不可伪造的授权依据。哈希不等于匿名化：标识、UUID、主机名、IP、MAC 均为敏感数据，本包、测试、打包检查和 benchmark 不记录或上传它们。

## Electron 与分发

安装已发布包（或 `pnpm pack` 生成的 tarball）：

```sh
pnpm add @hlw-ele/plugin-device
# 本地预编译包
pnpm add ./hlw-ele-plugin-device-0.1.0.tgz
```

发布包直接包含 Windows x64 预编译的 `dist/native/device.win32-x64-msvc.node`，无运行时 npm 依赖，也没有 `preinstall` / `install` / `postinstall` / `prepare` 编译脚本。安装和调用不需要 Rust、C++ Build Tools、Windows SDK 或单独安装 VC++ Redistributable。独立 Node 使用需已有 Node.js 20.19+；Electron exe 由宿主自带 Node 运行时，最终用户不需要另装 Node.js。

在 Electron 主进程调用，渲染端只消费必要字段。本包不添加 IPC。宿主负责将本包列为生产依赖并保留为 Vite / electron-vite 运行时 external；Electron Builder 必须收录包入口和完整 `dist/native`，并配置 `asarUnpack: ['**/*.node']`，确保 `.node` 实际位于可加载的磁盘路径。本包不修改宿主 external 或打包配置。

使用 N-API 8，不直接绑定 V8 ABI；要求 Windows 10+、x64 进程及支持 N-API 8 的 Node/Electron。`.cargo/config.toml` 为 `x86_64-pc-windows-msvc` 设置 `-C target-feature=+crt-static`，将 CRT 静态链接到本包原生模块，而非要求宿主提供 VC runtime DLL。二进制仍依赖 Windows 自带的系统 DLL/API，不是无系统依赖；不支持 Windows ARM64/ia32 进程。

每次发布必须检查真实产物的 `dumpbin /dependents dist/native/device.win32-x64-msvc.node`，不得出现 `VCRUNTIME*`、`MSVCP*`、`ucrtbase.dll` 或 `api-ms-win-crt-*` 等动态 CRT 依赖。现有 napi 3.14.2 / napi-build 2.6.0 / CLI 3.10.8 已通过静态 CRT 构建及本机 Node、Electron node mode 的采集验证；升级 Rust/napi 后仍需重复验证。本机导入表检查和 Electron node mode 并不等价于干净 Windows 10 环境或最终安装 exe 验收，后两项须由宿主发布流程执行。

首次调用通过 Promise 调度动态库的同步加载，因此加载错误也通过 Promise 拒绝；系统查询在原生后台任务执行。各 API 顺序读取，不是原子快照，不承诺硬截止时间，超时也不意味着原生任务可取消。后续成功调用仅复制首次快照，不再启动原生采集。

## 开发验证

仅源码开发、构建和发布机器需要 Rust MSVC（>= 1.88）、Visual Studio C++ Build Tools、Windows SDK、Node.js 及 pnpm 9。构建时不要用 `RUSTFLAGS` / `CARGO_ENCODED_RUSTFLAGS` 等环境配置覆盖静态 CRT 设置；依赖测试会检查最终 PE 导入表。

```sh
pnpm install
pnpm build
pnpm lint
pnpm typecheck
pnpm test
pnpm test:pack
pnpm bench
```

`build` 在 `target/native-*` 独立临时目录创建 napi 专用 manifest 和输出，使 napi 的文件事务也隔离于正式包目录；生成成功后通过重命名替换唯一的 `.node` 和生成声明，不先删除原产物。`src/types.d.ts` 始终来自 Rust `DeviceInfo`，声明脚本只规范多行 JSDoc，再打包 ESM/CJS 和声明，不手改字段。成功后只清理本次临时目录；失败保留目录并报告位置供检查，不自动删除未知备份或恢复旧事务。生成声明、`dist` 和 `target` 不手改、不提交。首次类型检查前先构建。

`pnpm pack --pack-destination target` / `pnpm publish` 的 `prepack` 会先执行构建，因此发布机器必须具备工具链；消费者安装 tarball 不执行构建。发布范围仅为 ESM/CJS 入口及声明、预编译 `.node`、README 和 package.json，不收录 Rust 源码、工具链、临时文件或事务备份。

测试检查精确键集合、required/optional 类型、无旧类型导出、多行字段 JSDoc、真实系统值、网卡地址一致性、原生 binding 实时变化、JS facade 快照复用与副本隔离及 SMBIOS/路由/电池解析；mock 测试验证首次并发仅采集一次、后续成功不再采集、采集失败重试、同步加载失败重试与副本隔离；固定合成 UUID 哈希向量覆盖 SMBIOS 两种字节序，缺失/全零/全 FF 标识必须报错。原生、打包和 benchmark 复用同一字段契约，拒绝旧四字段名称；ESM/CJS 声明包含旧名称不可访问的负向类型检查。原生、打包和 benchmark 契约均要求 `did`、`uuid` 为小写 64hex 必填字符串。测试断言不打印设备值。`test:pack` 解包真实 tarball，在独立临时目录通过包名校验 ESM/CJS 完整契约，随后清理自己的临时目录；只发布入口产物和 `.node`，不包含构建临时文件。

构建回归测试真实加载 `.node` 后重新构建，检查 napi 在隔离目录生成成功、正式替换因锁明确失败、原二进制和声明哈希不变，仅清理该测试自己创建的 staging。PE 依赖测试同时检查 x64 架构、普通导入与无延迟导入，限制为本模块使用的 Windows 10 系统 DLL。`test:pack` 严格检查 tarball 文件清单与无安装编译钩子。设置 `DEVICE_ELECTRON` 为现成 `electron.exe` 的完整路径后运行 `pnpm test:pack`，还会对解包后的同一二进制以 `ELECTRON_RUN_AS_NODE=1` 验证 ESM/CJS 采集契约，未设置时只验证 Node。

`bench` 采样 5 个新进程冷启动（导入及首次真实采集），另在预热成功后测量 15 次缓存快照复制，输出 `cachedSnapshot.cloneSnapshot` 计时，不代表后续原生采集耗时；仅输出计时、字段数量，子进程限时 30 秒。采集失败即失败，不输出敏感错误详情。

Windows 会锁定已加载的 `.node` 文件。若正式产物替换遇锁，保留原文件并报告，由使用者自行退出相关应用后重试，禁止自动结束用户进程。历史 `.napi-rs-filesystem-transaction.swp` 及 retired/backup 文件不属于新流程的清理范围；应先核对 owner/state、文件哈希和进程占用，再决定如何人工处理，不可盲删。
