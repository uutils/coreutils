#!/bin/bash
# spell-checker:ignore utmpdump gecos
# pinky is a tool to show logged-in users from utmp, and gecos fields from /etc/passwd.
# In GitHub Action *nix VMs, no accounts log in, even the "runner" account that runs the commands, and "system boot" entry is missing.
# The account also has empty gecos fields.
# To work around these issues for pinky (and who) tests, we create a fake utmp file with a
# system boot entry and a login entry for the GH runner account.
printf '%s\n%s\n%s' \
  '[2] [00000] [~~  ] [reboot  ] [~           ] [6.0.0-test                      ] [0.0.0.0        ] [2022-02-22T22:11:22,222222+0000]' \
  '[7] [999999] [tty2] [runner  ] [tty2        ] [                    ] [0.0.0.0        ] [2022-02-22T22:22:22,222222+0000]' \
  '[7] [00001] [tty3] [runner  ] [tty3        ] [                    ] [0.0.0.0        ] [2022-02-22T22:22:22,222222+0000]' \
  | sudo utmpdump -r -o /var/run/utmp
# ... and add a full name to each account with a gecos field but no full name.
sudo sed -i 's/:,/:runner name,/' /etc/passwd
# We also create a couple optional files pinky looks for
touch /home/runner/.project
echo "foo" > /home/runner/.plan
# add user with digital username for testing with issue #7787
echo 200:x:2000:2000::/home/200:/bin/bash | sudo tee -a /etc/passwd
echo 200:x:2000: | sudo tee -a /etc/group
