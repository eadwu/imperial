use libc::__u64;

/* clone3 `struct clone_args` prototype ported. */
#[repr(C)]
pub struct clone_args {
    pub flags: __u64, /* Flags bit mask */
    pub pidfd: __u64, /* Where to store PID file descriptor
                      (int *) */
    pub child_tid: __u64, /* Where to store child TID,
                          in child's memory (pid_t *) */
    pub parent_tid: __u64, /* Where to store child TID,
                           in parent's memory (pid_t *) */
    pub exit_signal: __u64, /* Signal to deliver to parent on
                            child termination */
    pub stack: __u64,      /* Pointer to lowest byte of stack */
    pub stack_size: __u64, /* Size of stack */
    pub tls: __u64,        /* Location of new TLS */
    pub set_tid: __u64,    /* Pointer to a pid_t array
                           (since Linux 5.5) */
    pub set_tid_size: __u64, /* Number of elements in set_tid
                             (since Linux 5.5) */
    pub cgroup: __u64, /* File descriptor for target cgroup
                       of child (since Linux 5.7) */
}
