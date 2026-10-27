// Waits for a line grep matches in a log, for lib.sh's await, expect and served: it looks again
// each time the file is written to, which kqueue tells it, and never on a timer, so a step costs
// the time the app takes and not the next tick of a poll. check.sh compiles it at its start.
//
//   logwait [-f <prefix>] [-p <pid>] <ms> <file> <pattern>
//
// Prints the first line from the file that matches <pattern>, a grep basic regular expression,
// and exits 0. With -f, it looks only from the first line that starts with <prefix>, as
// `sed -n '/^<prefix>/,$p'` does. Exits 1 if no line matches within <ms> milliseconds, and 2 if
// process <pid> exits first, once the lines it wrote have been read.
#include <errno.h>
#include <fcntl.h>
#include <libgen.h>
#include <spawn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/event.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

extern char** environ;

static long long now_ms(void) {
  struct timespec now;
  clock_gettime(CLOCK_MONOTONIC, &now);
  return now.tv_sec * 1000LL + now.tv_nsec / 1000000;
}

// Where the first line starting with |prefix| begins, or -1 while there's none. |scanned| keeps
// how far the file has been read, to the end of its last whole line.
static off_t find_prefix(int fd, const char* prefix, off_t* scanned) {
  size_t length = strlen(prefix);
  char buffer[1 << 16];
  // The start of the line being read, and its first bytes, up to the prefix's length.
  off_t line = *scanned;
  char head[256];
  size_t have = 0;
  if (length > sizeof head) {
    return -1;
  }
  off_t at = *scanned;
  for (;;) {
    ssize_t got = pread(fd, buffer, sizeof buffer, at);
    if (got <= 0) {
      return -1;
    }
    for (ssize_t i = 0; i < got; i++, at++) {
      if (buffer[i] == '\n') {
        if (have == length && memcmp(head, prefix, length) == 0) {
          return line;
        }
        line = at + 1;
        *scanned = line;
        have = 0;
      } else if (have < length) {
        head[have++] = buffer[i];
      }
    }
  }
}

// Runs grep -m1 on the file from |from| on, with its output on ours, and returns whether it
// matched.
static int grep_from(int fd, off_t from, const char* pattern) {
  if (lseek(fd, from, SEEK_SET) < 0) {
    return 0;
  }
  posix_spawn_file_actions_t actions;
  posix_spawn_file_actions_init(&actions);
  posix_spawn_file_actions_adddup2(&actions, fd, STDIN_FILENO);
  char* argv[] = {"grep", "-m1", "-e", (char*)pattern, NULL};
  pid_t child;
  int status = 0;
  int spawned = posix_spawnp(&child, "grep", &actions, NULL, argv, environ);
  posix_spawn_file_actions_destroy(&actions);
  if (spawned != 0) {
    fprintf(stderr, "logwait: can't run grep: %s\n", strerror(spawned));
    exit(3);
  }
  while (waitpid(child, &status, 0) < 0 && errno == EINTR) {
  }
  return WIFEXITED(status) && WEXITSTATUS(status) == 0;
}

int main(int argc, char** argv) {
  const char* prefix = NULL;
  pid_t pid = 0;
  int option;
  while ((option = getopt(argc, argv, "f:p:")) != -1) {
    if (option == 'f') {
      prefix = optarg;
    } else if (option == 'p') {
      pid = (pid_t)atoi(optarg);
    } else {
      return 3;
    }
  }
  if (argc - optind != 3) {
    fprintf(stderr, "usage: logwait [-f <prefix>] [-p <pid>] <ms> <file> <pattern>\n");
    return 3;
  }
  long long deadline = now_ms() + atoll(argv[optind]);
  const char* path = argv[optind + 1];
  const char* pattern = argv[optind + 2];

  int queue = kqueue();
  struct kevent change;
  struct kevent event;
  struct timespec wait;
  int exited = 0;
  if (pid > 0) {
    EV_SET(&change, pid, EVFILT_PROC, EV_ADD | EV_ONESHOT, NOTE_EXIT, 0, NULL);
    // A process that has exited, even one not yet reaped, can't be watched.
    exited = kevent(queue, &change, 1, NULL, 0, NULL) < 0;
  }
  // A run waits right after it starts the app, whose log the shell may not have made yet: then
  // what wakes it is the file's directory changing.
  int fd;
  while ((fd = open(path, O_RDONLY)) < 0) {
    long long left = deadline - now_ms();
    if (errno != ENOENT || exited || left <= 0) {
      return exited ? 2 : 1;
    }
    char directory[1024];
    strncpy(directory, path, sizeof directory - 1);
    directory[sizeof directory - 1] = 0;
    int watched = open(dirname(directory), O_EVTONLY);
    if (watched < 0) {
      return 3;
    }
    EV_SET(&change, watched, EVFILT_VNODE, EV_ADD | EV_CLEAR, NOTE_WRITE, 0, NULL);
    kevent(queue, &change, 1, NULL, 0, NULL);
    if (access(path, F_OK) != 0) {
      wait.tv_sec = left / 1000;
      wait.tv_nsec = left % 1000 * 1000000;
      if (kevent(queue, NULL, 0, &event, 1, &wait) > 0 && event.filter == EVFILT_PROC) {
        exited = 1;
      }
    }
    close(watched);
  }
  // Watched before the first look, so a line written while grep reads isn't missed.
  EV_SET(&change, fd, EVFILT_VNODE, EV_ADD | EV_CLEAR, NOTE_WRITE | NOTE_EXTEND, 0, NULL);
  kevent(queue, &change, 1, NULL, 0, NULL);

  off_t scanned = 0;
  off_t from = prefix ? -1 : 0;
  for (;;) {
    if (from < 0) {
      from = find_prefix(fd, prefix, &scanned);
    }
    if (from >= 0 && grep_from(fd, from, pattern)) {
      return 0;
    }
    if (exited) {
      return 2;
    }
    long long left = deadline - now_ms();
    if (left <= 0) {
      return 1;
    }
    wait.tv_sec = left / 1000;
    wait.tv_nsec = left % 1000 * 1000000;
    if (kevent(queue, NULL, 0, &event, 1, &wait) > 0 && event.filter == EVFILT_PROC) {
      exited = 1;
    }
  }
}
