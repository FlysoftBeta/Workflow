#define main proxy_guard_cli_main
#include "../proxy_guard.c"
#undef main
#include <assert.h>

static void attribute(struct nlmsghdr *header, unsigned short type, const void *value, size_t size) {
    struct rtattr *attr = (struct rtattr *)((char *)header + NLMSG_ALIGN(header->nlmsg_len));
    attr->rta_type = type; attr->rta_len = RTA_LENGTH(size);
    memcpy(RTA_DATA(attr), value, size);
    header->nlmsg_len = NLMSG_ALIGN(header->nlmsg_len) + RTA_ALIGN(attr->rta_len);
}

static int sample_rule(uint32_t priority, uint32_t table, const char *interface) {
    unsigned char bytes[512] = {0};
    struct nlmsghdr *header = (struct nlmsghdr *)bytes;
    header->nlmsg_len = NLMSG_LENGTH(sizeof(struct fib_rule_hdr));
    struct fib_rule_hdr *rule = NLMSG_DATA(header);
    rule->family = AF_INET; rule->action = FR_ACT_TO_TBL;
    attribute(header, FRA_PRIORITY, &priority, sizeof(priority));
    attribute(header, FRA_TABLE, &table, sizeof(table));
    attribute(header, FRA_IIFNAME, interface, strlen(interface) + 1);
    return scoped_rule(header);
}

int main(void) {
    uint64_t value = 0;
    char complicated[] = "111 (hello ) odd\nname) S 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 987654321 23";
    assert(parse_start_time(complicated, &value) && value == 987654321);
    char missing[] = "111 (hello) S 1 2 3";
    assert(!parse_start_time(missing, &value));
    char negative[] = "111 (hello) S 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 -12";
    assert(!parse_start_time(negative, &value));
    char overflow[] = "111 (hello) S 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 18446744073709551616";
    assert(!parse_start_time(overflow, &value));
    cleanup.table = 9500; cleanup.priority = 9500;
    strcpy(cleanup.device, "workflow-tun");
    assert(sample_rule(9501, 9500, "lo") == 1);
    assert(sample_rule(9501, 9500, "workflow-tun") == 1);
    assert(sample_rule(9501, 9500, "Meta") == 2);
    assert(sample_rule(9501, 3000, "lo") == 2);
    assert(sample_rule(9700, 9500, "lo") == 2);
    assert(sample_rule(9700, 9700, "lo") == 0);
    uint32_t integer;
    assert(uint_arg("9500", &integer) && integer == 9500);
    assert(!uint_arg("-1", &integer) && !uint_arg("4294967296", &integer));
    puts("PASS: /proc identity parser and exact TUN policy-rule cleanup scope");
    return 0;
}
