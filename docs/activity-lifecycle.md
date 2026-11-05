# 指令库生命周期（第二批）

新增命令全部依赖 inproc 桥和运行中的影刀客户端。首次调用自动注入，显式 `XBOT_INPROC_ENDPOINT` 只探测指定监听器。桥不可用时硬失败，不调用官方接口兜底。当前只有隔离测试证据；本批真实影刀验收由操作者按下方清单执行。

```powershell
xbot-cli activity probe
xbot-cli activity current
xbot-cli activity install-state
xbot-cli activity create --name '验收指令库' --code activity_batch2_acceptance --description '第二批隔离验收'
xbot-cli activity finish-create --app-id $libraryId --name '验收指令库' --code activity_batch2_acceptance --description '第二批隔离验收'
xbot-cli activity create-consumer --name '验收消费者' --description '第二批安装与升级验证'
xbot-cli activity install --app-id $consumerId --activity-code activity_batch2_acceptance --version $v1 --name '验收指令库'
xbot-cli activity install --app-id $consumerId --activity-code activity_batch2_acceptance --version $v2 --name '验收指令库' --upgrade
xbot-cli activity metadata-apply --app-id $libraryId --root $robotPath --manifest metadata.json
```

`create` 使用客户端 `activity.zip` 模板，`create-consumer` 使用 `blank.zip`。两者要求 Studio 已关闭、管理器明确空闲。`finish-create` 只对给定 UUID 的新 activity 补全配置，拒绝将已有不同名称或语义编码的库改名。名称、简介不能为空；新建编码在 activity_ 后至少两个字符，拒绝默认随机十六进制串。成功后 CLI 自动回读当前 UUID、类型、名称、简介和编码，返回 `data.verified=true`。

`install` 要求指定工作区处于编辑模式、引擎明确空闲。已有不同版本需要 `--upgrade`；升级不存在的直接依赖会被拒绝。`--version` 传真实发布版本字符串（数字开头），不要传 versionId 或 `latest`。同版本可重复调用，但 CLI 从不自动重试。最终结果包含目标版本、安装文件版本和依赖清单，并再次回读当前工作区，核对本次之外的直接依赖保持不变。`activity current`、`install-state`、`finish-create` 和 `install` 支持 `--expect-app`，通过 inproc 核对目标。

写型命令支持 `--dry-run`，只解析和校验输入，不联系影刀。`metadata-apply --manifest -` 可从 stdin 读取。预览成功仍需运行时前置校验。

## 装库中断的根因与处理

旧 0.4.1 在原生安装返回后立即检查 `Workspace.InternalDependencies`，依赖尚未刷新便报错，后续保存被跳过。升级还有第二个条件：工作区 `_documentChanged` 和 `HasChanged` 为 false，单独调用 Save 会保留原来的依赖版本。

只读参考树的 `docs/activity-editing.md` 与修复记录 `E:/zhongzhuan/artifacts/xbot-cli-install-fix-20261104/修复记录.md` 记录了下载后提前报错和仅补 Save 仍保留旧版本的复现。当前参考树已是 0.4.2，其修正算法也已随第一批 payload 入库；第二批复用并测试这条算法，不重新引入早期检查：

1. 等待 InstallActivity / UpgradeActivity 的原生 Task 完成。
2. 用仓库 VerifyInstalled 核对下载版本。
3. NotifyActivityAdd，再 RaiseWorkspaceChanged(ActivityInstalled / ActivityUpdated, true)，使原生文档进入需要保存的状态。
4. Save，等待 WaitPythonTaskCenterIdle 成功；等待期间不阻塞 UI 线程。
5. 在 UI ContextIdle 回调中再次核对同一个 Workspace、UUID、编辑模式和空闲引擎，再 Save。
6. 核对内存依赖、磁盘 package.json 依赖、xbot_extensions 中的安装版本，以及无关直接依赖。

通知、脏标记、保存和依赖任务等待的顺序现由可执行的隔离回归锁定。真实客户端的异步行为仍须执行安装及升级实机验收。

## 元数据清单与备份

元数据写入在影刀进程内执行，要求整个 Studio 已关闭且管理器明确空闲。`--root` 是目标 `xbot_robot` 的绝对路径，现有 package 的 UUID 和 robot_type 必须匹配。清单的键是包内路径，值是完整 JSON 对象或 XML/初始化文件文本：

| 允许路径 | 值 |
| --- | --- |
| package.json | 完整对象；uuid、robot_type、activity_code 保持原值 |
| prototype.block.json | 完整对象 |
| prototype.activity.xml | 完整文本 |
| __init__.py | 初始化/注册文本 |
| .dev/workspace.state.json | 完整对象 |
| .dev/单个名称.flow.json | 完整对象 |

普通 CodeFlow `.py`、跨目录路径、反斜杠/冒号变体及 `.dev` 下其他文件会被拒绝。先验证完整写集和所有内容类型，再备份所有已存在的目标文件，然后写入并逐项比对磁盘字节。成功返回 `data.result.backup`、`files`、`verification=disk_readback`。磁盘写入后须正常重开、保存编译，再核对原型、公开清单与调用绑定。

写入或回读失败可能留下部分变更，错误保留备份路径和各文件原来是否存在；先按备份核对恢复，不自动重发。未保存的编辑器不能通过关闭流程强行丢弃，先处理已有工作。

## 其它旧能力与守卫

| 旧操作 | 本批路径与范围 |
| --- | --- |
| publish | 使用现有 `app publish <uuid> --log '说明' --yes`，含现有版本回读；不新增同名 activity 别名 |
| sync | 使用现有 `app sync`，会上传并关闭 Studio；不新增重复同步实现 |
| backup | metadata-apply 自动备份本次修改的原件；本批不增加通用整库备份命令，整库归档涉及不同的目录/生成物范围 |
| Studio 编辑 | 使用现有 flow 命令；源码使用 `flow code write/edit` |
| 研究/目录/采集与 client-api | 不在本批接入范围，未增加对外命令 |

新 activity 写型命令全部经 inproc；上述已有 app 命令保留原有传输与确认规则。没有新增通用 studio-call 或 reload 逃生舱。指令库工作区的 app.reload、codeflow.write、codeflow.edit 仍由既有收口拒绝。不要用同步关闭绕过已知的内存公开标志漂移，发现漂移先受控校准。

错误输出沿用 BackendError：输入错误退出 2；桥未就绪退出 4；CLI 的 UUID/模式冲突及 Studio 未关闭的 STUDIO_BUSY 退出 5；原生引擎运行/暂停/未知状态拒绝由 INPROC_FAILED 返回，退出 1。写入回执丢失 `INPROC_WRITE_UNKNOWN` 退出 6 且不可自动重试；返回后核验失败 `ACTIVITY_VERIFY_FAILED` 退出 1、保留已返回的结果且不可自动重试。安装中途失败保留服务端阶段信息，失败不执行自动回滚。

## 实机验收清单（待操作者执行）

使用本分支构建的 exe，隔离测试库和消费者；不要复用业务应用。PowerShell 中先设置 `$cli` 为 exe 的绝对路径，清除指向 mock 的 `XBOT_INPROC_ENDPOINT`。保存每条 JSON 和退出码；遇到结果未知先回读。

1. **发现与桥离线。** `& $cli activity --help`、`& $cli schema activity install --json`：列出新命令、inproc 依赖及参数。先把 `XBOT_INPROC_ENDPOINT` 临时设为 `http://127.0.0.1:1`，执行 `activity create --name '验收指令库' --code activity_batch2_acceptance --description '第二批隔离验收' --json`，预期 INPROC_OFFLINE、退出 4 且没有新库；再清除此变量。
2. **探测。** 关闭 Studio 后运行 `& $cli activity probe --json`，检查 ok、hasStudio=false、isBusy=false。`activity current` 在没有工作区时应失败，不能报告已创建。不要把探测成功当作建库成功。
3. **创建 activity。** 执行上文 create 命令并保存 JSON，记录 `data.current.uuid` 为 `$libraryId`、`robotPath` 为 `$robotPath`。预期 verified=true、robotType=Activity、activityCode 正确，package UUID/类型/编码一致；在已打开的 Studio 再 create 应拒绝且不新增应用。
4. **完成建库。** 对刚创建的同一 UUID 用同样名称、编码和简介执行 finish-create，预期 verified=true、UUID 不变。替换成另一 UUID、另一个已有库名称/编码分别执行，预期拒绝；核对库没有被重命名。创建后续失败恢复也使用此入口，先 current 获取实际 UUID，禁止重复 create。
5. **准备可发布的最小库。** 用现有 `flow create`、`flow params set` 和 `flow code write` 做一条无业务副作用的公开验收指令，带显式输入、输出断言及版本标记。公开清单按 package/prototype 核对；先 `app save`，再 `app sync`。调用 `app publish $libraryId --log '第二批验收 v1' --yes --json`，从返回版本信息记录真实版本 `$v1`；另行准备逻辑标记不同的 `$v2`。发布与创建流程的完整参数以实际 `schema` 为准。
6. **元数据应用。** Studio 关闭后，从 `$robotPath/package.json` 读完整当前对象，仅修改测试用 description，保存为 `{ "package.json": 完整修改后对象 }` 的 manifest。执行 `activity metadata-apply --app-id $libraryId --root $robotPath --manifest metadata.json --json`，预期 verified=true、disk_readback、有备份目录；备份内容是旧值、目标文件是新值。清单加入 main.py、../ 路径或修改 activity_code 时应拒绝，写集其它文件保持原样。保持 Studio 打开执行同一命令也应拒绝。
7. **重开核对。** `& $cli app open $libraryId --json`，`& $cli app save --json`，再读 package、完整原型和公开清单，检查元数据持久化、UUID/编码不变。确认无漂移后 `app sync` 关闭 Studio。
8. **创建消费者。** 执行上文 create-consumer 命令，记录 `data.current.uuid` 为 `$consumerId`。预期 verified=true、robotType 为普通应用类型、没有伪造 activity 类型；`activity current` 再读同一 UUID。
9. **新装。** `activity install-state --json` 明确空闲后执行上文安装 `$v1` 命令，预期 verified=true、result.installedVersion=$v1、current.dependencies 含唯一 `activity_batch2_acceptance==$v1`。对比磁盘 internaldependencies、xbot_extensions/activity_batch2_acceptance/package.json 的版本。运行/暂停消费者时执行 install 应拒绝，保持原状态；错误 UUID 应拒绝。
10. **升级。** 在消费者中给 v1 公开指令配置一条实际调用，显式输入如 beta，并记录块 ID/绑定/禁用状态和输出。先调用 v2 不带 --upgrade，应拒绝；带 --upgrade 应成功，内存、磁盘和安装文件均为 `$v2`，其它直接依赖、原有块 ID 和调用绑定不变。同版本重复 install/upgrade 不得出现混合 pin。升级不存在的依赖应拒绝。
11. **运行与重开。** 用现有 `flow run <实际验收flow_id> --timeout 60 --json`，断言显式输入和 v2 标记的输出；如返回未结束的 run_id，用 `flow wait <run_id> --timeout 600 --json` 续等，用 `flow logs --json` 查日志。保存、同步关闭、正常重开消费者后再 current、核对三处版本并重新运行，同样断言输出。
12. **旧守卫回归。** 打开指令库，在 `tool studio call app.reload '{}'`、`tool studio call codeflow.write '{}'`、`mcp call codeflow.edit '{}'` 上检查 MCP_TOOL_BLOCKED；`flow code edit` 改注释仍保留表单、参数及调用方 beta。普通应用的原有 app 操作保持原行为。

任何写型请求超时/回执丢失均先 current、install-state 和磁盘核对，确认实际状态后再决定恢复方式。复杂 Python/嵌套依赖、其它客户端版本、发布后的任意参数契约迁移需要各自验收，本批自动化测试没有声明这些能力已通过。
