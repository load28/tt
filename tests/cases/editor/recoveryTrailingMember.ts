type Status = { kind: "Idle" } | { kind: "Busy" };
declare const subject: Status;
const broken = subject.
const a = subject.kind === "Idle" ? 1 : 2;
const b = subject.kind === "Idle" ? 3 : 4;
const c = subject.kind === "Idle" ? 5 : 6;
const later = "정상";
const wrong: number = "wrong";
later./*member*/toUpperCase();
/*later*/later;
