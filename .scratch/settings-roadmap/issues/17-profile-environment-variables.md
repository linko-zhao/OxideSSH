# 17: Profile 环境变量

**What to build:** 用户可在 Profile 中编辑要发送给远端会话的环境变量；服务端接受后，远端应用可读取对应值。

**Blocked by:** None (can start immediately).

**Status:** merged-tested

- [ ] 环境变量可在 Profile 创建和编辑流程中增删改并持久化。
- [ ] 建立会话时发送配置的环境变量请求，变量名和值保持完整。
- [ ] 非法变量名无法保存，不生成无效请求。
- [ ] 服务端未接受变量时，界面不伪报远端已应用。
- [ ] 缺字段的旧 Profile 可加载，`schemaVersion` 保持 1。

## Progress — 2026-10-25

- 完整实现已提交为 `ffb12df`、`e1ba5f2`，分支 `ticket/17-profile-environment-variables` 保留。
- 串行合并时 Profile 模型、会话处理和桌面表单等多处冲突；按规则中止，未运行 workspace check。
- integration 保持 `fac1060`，后续解决冲突后重试。
- 后续已解决 Profile、表单和会话冲突并合入 integration；环境变量在启动命令/交互 Shell 前发送，workspace check、SSH session 19 项及完整 136 项测试通过。
