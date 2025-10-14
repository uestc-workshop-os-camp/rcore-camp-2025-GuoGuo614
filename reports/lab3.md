## 简单总结
- 确实很简单，没有太多可以说的。
- 迁移上一章的内容，本章主要是文件结构变化，原本把mmap的内部逻辑写到task模块，现在看来没有必要了，直接插入映射就可以了。
- 进程创建就是使用elf数据创建进程，实际上不是fork+exec，因为不需要复制父进程地址空间。
  - 原本结合了一下两个系统调用，除掉了重复的部分，后来发现直接调用new创建TCB就可以了，注意一下父子进程的维护。
- stride调度算法，看懂之后给TCB加两个字段即可，在fetch_task取出下一个进程的时候进行一个遍历就好了。
## 问答题
- stride算法深入，进行一个溢出的判断。
- 不是，p2执行一个时间片后会增加stride的值，250 + 10 = 260，溢出为4，此时小于p1的stride，故仍然是p2的回合。
- STRIDE_MIN是0，而prio>=2，STRIDE_MAX = BIG_STRIDE / prio，当然<= BigStride / 2了。
- 考虑溢出的话，用wrapping_sub修复一下就可以了。
```
use core::cmp::Ordering;

struct Stride(u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let diff = self.0.wrapping_sub(other.0);
        if diff == 0 {
            None
        } else if diff < 128 {
            Some(Ordering::Greater)
        } else {
            Some(Ordering::Less)
        }
    }
}

impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}
```

## 荣誉准则
1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

无

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

无

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。