# Lab3 报告

## 实现的功能

### 实现目标

实现系统调用 `sys_trace`，用于追踪当前任务的系统调用历史信息，并支持查询与修改。该系统调用通过 `trace_request` 参数区分三种功能：

| `trace_request` | 功能描述 |
|---|---|
| 0 | 将 `id` 视为用户地址，读取该地址上的 1 字节并返回，`data` 忽略 |
| 1 | 将 `id` 视为可写用户地址，将 `data` 的低 8 位写入该地址，返回 0 |
| 2 | 查询当前任务对编号为 `id` 的系统调用的累计调用次数，返回次数（本次查询也计入统计） |
| 其他值 | 返回 -1 |

### 实现方法

在 `TaskControlBlock` 结构体中新增 `syscall_cnt: [usize; MAX_SYSCALL_NUM]` 数组，用于记录各系统调用的累计调用次数。

为满足 `sys_trace` 第三种功能的查询需求，在 `task/mod.rs` 中添加了以下两个对外暴露的方法：

- `get_current_syscall_cnt(id)` —— 查询当前任务对指定系统调用的累计调用次数
- `inc_current_syscall(syscall_id)` —— 将指定系统调用的计数加一

在 `TaskControlBlock` 上实现了对应的内部方法 `inc_syscall` 和 `get_syscall_cnt`，作为上述外部方法的底层接口。

由于系统调用仅在 `trap_handler` 中处理，因此在调用实际 `syscall` 分发之前，先调用 `inc_current_syscall(syscall_id)` 完成计数递增。

`trace_request` 为 0 和 1 时，对传入参数进行类型断言后，使用 `unsafe` 直接读写内存即可完成操作。

---

## 简答作业

### 测试环境

- **SBI**: RustSBI version 0.3.0-alpha.2 (RustSBI-QEMU Version 0.2.0-alpha.2)
- **QEMU**: riscv64 virt machine

### 三个 bad 测例的出错行为

运行 `ch2b_bad_address`、`ch2b_bad_instructions`、`ch2b_bad_register` 三个测例后，内核均捕获异常并终止对应应用程序，具体行为如下：

| 测例 | 触发的异常 | 出错行为 |
|---|---|---|
| `ch2b_bad_address` | `IllegalInstruction` | 用户程序尝试向地址 `0x0` 写入数据，触发异常，内核打印错误信息后杀死该进程 |
| `ch2b_bad_instructions` | `IllegalInstruction` | 用户程序在 U 态执行 `sret` 指令（S 态特权指令），触发 `IllegalInstruction` 异常，内核杀死该进程 |
| `ch2b_bad_register` | `IllegalInstruction` | 用户程序在 U 态执行 `csrr sstatus` 读取 S 态寄存器，触发 `IllegalInstruction` 异常，内核杀死该进程 |

以上三个测例均验证了：正确进入 U 态后，使用 S 态特权指令或访问 S 态寄存器会触发异常，内核通过 `trap_handler` 捕获并终止相应用户程序。

---

### `__alltraps` 和 `__restore` 函数分析

#### L40：刚进入 `__restore` 时，sp 代表什么值？`__restore` 的两种使用情景是什么？

刚进入 `__restore` 时，`sp` 指向**内核栈**上已保存的 `TrapContext` 结构体的起始位置。

`__restore` 的两种使用情景：

1. **trap 处理完成后返回用户态**：`trap_handler` 执行完毕后返回 `TrapContext` 的指针（即 `sp`），随后执行 `__restore` 恢复上下文并返回用户态。
2. **首次启动用户任务**：`run_first_task` 通过 `__switch` 切换到已初始化好的任务上下文，该上下文的 `__restore` 入口地址被设置为返回地址，从而首次进入用户态。

---

#### L43-L48：这几行汇编代码特殊处理了哪些寄存器？对进入用户态有何意义？

```asm
ld t0, 32*8(sp)
ld t1, 33*8(sp)
ld t2, 2*8(sp)
csrw sstatus, t0
csrw sepc, t1
csrw sscratch, t2
```

这三个寄存器及其意义：

- **`sstatus`(t0)**：保存了处理器状态寄存器。其中 `SPP` 位决定了 `sret` 执行后返回的特权级（0 表示 U 态，1 表示 S 态）。恢复该寄存器确保返回用户态时特权级正确切换。
- **`sepc`(t1)**：保存了异常程序计数器，即 `sret` 执行后 `pc` 要跳转到的地址——也就是用户程序被中断时的下一条指令地址。恢复该寄存器确保用户程序从正确的位置继续执行。
- **`sscratch`(t2)**：恢复为用户栈指针。`sscratch` 在 trap 处理中充当用户态与内核态栈指针的交换媒介，恢复它为下一次 trap 时的正确交换做好准备。

这三者不能通过通用寄存器恢复路径处理，因为它们是 CSR 寄存器，必须用 `csrw` 指令显式写入。

---

#### L50-L56：为何跳过了 x2 和 x4？

```asm
ld x1, 1*8(sp)
ld x3, 3*8(sp)
.set n, 5
.rept 27
   LOAD_GP %n
   .set n, n+1
.endr
```

- **`x2`（`sp`，栈指针）**：不能此时恢复，因为当前 `sp` 正指向 `TrapContext` 所在的内核栈位置，用于寻址其他寄存器的恢复。`sp` 的恢复留到后面通过 `addi sp, sp, 34*8` 释放 `TrapContext` 空间后再通过 `csrrw sp, sscratch, sp` 完成。
- **`x4`（`tp`，线程指针）**：在用户态应用程序中不使用该寄存器，因此无需保存和恢复。

---

#### L60：该指令之后，sp 和 sscratch 中的值分别有什么意义？

```asm
csrrw sp, sscratch, sp
```

执行该指令前：
- `sp` → 内核栈（已释放 TrapContext，即用户态的内核栈顶）
- `sscratch` → 用户栈指针

执行该指令后：
- `sp` → 用户栈指针
- `sscratch` → 内核栈指针

这样，`sret` 执行后进入用户态时，`sp` 为用户栈，而 `sscratch` 中保存着内核栈指针，供下次 trap 进入 `__alltraps` 时交换使用。

---

#### `__restore` 中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？

状态切换发生在 **`sret`** 指令。

`sret`执行时，硬件自动完成以下操作：
1. 将 `pc` 设置为 `sepc` 的值，即跳转回用户程序被中断的位置
2. 将当前特权级设置为 `sstatus.SPP` 字段所指示的级别（对于从用户态陷入的情况，`SPP = 0`，即 U 态）
3. 将 `sstatus.SIE` 设置为 `sstatus.SPIE` 的值，并将 `sstatus.SPIE` 置 1

因此 `sret` 执行后，处理器从 S 态切换回 U 态，继续执行用户程序。

---

#### L13：该指令之后，sp 和 sscratch 中的值分别有什么意义？

```asm
csrrw sp, sscratch, sp
```

（这是 `__alltraps` 的入口第一条指令）

执行该指令前（刚从 U 态 trap 进入 S 态）：
- `sp` → 用户栈指针
- `sscratch` → 内核栈指针

执行该指令后：
- `sp` → 内核栈指针
- `sscratch` → 用户栈指针

这样，后续代码可以在内核栈上安全地保存用户态上下文，同时 `sscratch` 中保存着用户栈指针供后续恢复使用。

---

#### 从 U 态进入 S 态是哪一条指令发生的？

从 U 态进入 S 态由 **`ecall`**（Environment Call）指令触发。

当用户程序执行 `ecall` 时，硬件会：
1. 将当前特权级保存到 `sstatus.SPP`
2. 将 `pc`（`ecall` 的下一条指令地址）保存到 `sepc`
3. 将 trap 原因写入 `scause`
4. 将当前特权级切换为 S 态（由QEMU模拟的硬件完成）
5. 将 `pc` 设置为 `stvec` 寄存器中保存的 trap 处理入口地址（即 `__alltraps`）

此外，非法指令异常（如在 U 态执行 `sret`）和存储访问异常（如写入非法地址）等也会触发从 U 态到 S 态的 trap。

## 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

独立完成

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

仅参考了项目源码

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。