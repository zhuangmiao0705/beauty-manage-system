# 项目分层说明

## 前端目录

- `src/auth/`：登录状态、鉴权服务及浏览器演示账号适配
- `src/components/`：可复用界面组件；按业务域继续建立子目录
- `src/composables/`：跨页面复用的响应式业务流程
- `src/config/`：应用信息、选项字典、导航等稳定配置
- `src/data/`：门店数据仓库，以及浏览器本地数据适配
- `src/platform/`：Tauri 等运行平台能力封装
- `src/router/`：路由表与路由权限守卫
- `src/services/`：不依赖界面的业务计算，例如营收报表和员工薪酬计算
- `src/styles/`：按职责拆分的全局样式模块
- `src/types/`：领域模型与鉴权模型
- `src/utils/`：日期、格式化、通知和文件导出等无业务状态工具
- `src/views/`：路由页面，只负责组合组件和编排页面交互

## 后端目录

- `src-tauri/src/models.rs`：数据库输入输出模型
- `src-tauri/src/state.rs`：数据库连接、备份目录和登录会话状态
- `src-tauri/src/security.rs`：密码哈希与校验
- `src-tauri/src/database.rs`：迁移、初始化、查询与快照组装
- `src-tauri/src/commands/auth.rs`：登录、改密、账号及授权命令
- `src-tauri/src/commands/salon.rs`：会员、交易、服务和员工命令
- `src-tauri/src/commands/packages.rs`：套盒配置、购买和原子消耗命令
- `src-tauri/src/commands/appointments.rs`：预约登记、时间冲突、状态流转和完成服务命令
- `src-tauri/src/commands/payroll.rs`：考勤、员工独立薪酬规则和工资计算命令
- `src-tauri/src/commands/backup.rs`：备份与恢复命令
- `src-tauri/src/lib.rs`：只负责组装模块并启动 Tauri

## 依赖方向

页面可以依赖组件、组合函数、服务、仓库、配置和工具；组件不应反向依赖页面。数据仓库通过 `platform` 判断运行环境，页面不直接调用 Tauri。后端命令依赖数据库与安全模块，数据库模块不依赖命令模块。

新增固定选项时优先放入 `config`，新增纯计算逻辑时优先放入 `services` 或 `utils`，新增跨页面状态流程时优先放入 `composables`，避免继续把业务逻辑堆入页面文件。

会员余额由“实付本金余额 + 赠送余额”组成。普通项目消费按赠送余额优先扣减，套盒余额购买只允许扣减实付本金；浏览器仓库和 SQLite 命令必须保持相同规则。员工薪酬配置按员工独立保存，普通员工快照不返回薪酬敏感字段。

预约登记不提前产生营收、提成或套盒扣次；预约进入“服务中”后才能完成。完成普通预约时生成普通服务记录，完成套盒预约时在同一事务内扣减套盒次数、写入消耗流水并生成套盒服务记录。
