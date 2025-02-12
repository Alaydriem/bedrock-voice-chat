export function initExample1() {
  return {
    usersData: [
      {
        id: "1",
        avatar_url: "images/200x200.png",
        name: "John Doe",
        age: "24",
        phone: "443-893-2318",
        role: "Superadmin",
        role_bg:
          "bg-secondary/10 text-secondary dark:bg-secondary-light/15 dark:text-secondary-light",
        status: true,
      },
      {
        id: "2",
        avatar_url: "images/200x200.png",
        name: "Sabina Mores",
        age: "26",
        phone: "442-893-2318",
        role: "Author",
        role_bg: "bg-info/10 text-info dark:bg-info/15",
        status: false,
      },
      {
        id: "3",
        avatar_url: "images/200x200.png",
        name: "Tom Robert",
        age: "34",
        phone: "443-893-2318",
        role: "Admin",
        role_bg:
          "bg-secondary/10 text-secondary dark:bg-secondary-light/15 dark:text-secondary-light",
        status: true,
      },
      {
        id: "4",
        avatar_url: "images/200x200.png",
        name: "Nolan Doe",
        age: "28",
        phone: "443-893-2318",
        role: "Author",
        role_bg: "bg-info/10 text-info dark:bg-info/15",
        status: true,
      },
    ],
    allSelected: false,
    userIds: [],
    updateCheckAll() {
      this.allSelected = this.usersData.length === this.userIds.length;
    },
    selectAll() {
      this.userIds = [];
      if (this.allSelected) {
        for (user in this.usersData) {
          this.userIds.push(this.usersData[user].id.toString());
        }
      }
    },
  };
}

export function initGridTableExapmle() {
  return {
    table: null,
    config: {
      columns: [
        {
          id: "id",
          name: "ID",
          formatter: (cell) => Gridjs.html(`<span class="mx-2">${cell}</span>`),
        },
        {
          id: "name",
          name: "Name",
          formatter: (cell) =>
            Gridjs.html(
              `<span class="text-slate-700 dark:text-navy-100 font-medium">${cell}</span>`
            ),
        },
        {
          id: "avatar_url",
          name: "Avatar",
          sort: false,
          formatter: (cell) =>
            Gridjs.html(`<div class="avatar flex">
                                    <img class="rounded-full" src="${cell}" alt="avatar">
                                </div>`),
        },
        {
          id: "email",
          name: "Email",
        },
        {
          id: "phone",
          name: "Phone Number",
        },
        {
          name: "Actions",
          sort: false,
          formatter: () =>
            Gridjs.html(`<div class="flex justify-center space-x-2">
                            <button @click="editItem" class="btn size-8 p-0 text-info hover:bg-info/20 focus:bg-info/20 active:bg-info/25">
                                <i class="fa fa-edit"></i>
                            </button>
                            <button @click="deleteItem" class="btn size-8 p-0 text-error hover:bg-error/20 focus:bg-error/20 active:bg-error/25">
                                <i class="fa fa-trash-alt"></i>
                            </button>
                        </div>`),
        },
      ],
      data: [
        {
          id: "1",
          name: "John",
          email: "john@example.com",
          phone: "(01) 22 888 4444",
          avatar_url: "images/200x200.png",
        },
        {
          id: "2",
          name: "Doe",
          email: "thedoe@example.com",
          phone: "(33) 22 888 4444",
          avatar_url: "images/200x200.png",
        },
        {
          id: "3",
          name: "Nancy",
          email: "nancy@example.com",
          phone: "(21) 33 888 4444",
          avatar_url: "images/200x200.png",
        },
        {
          id: "4",
          name: "Clarke",
          email: "clarke@example.com",
          phone: "(44) 33 888 4444",
          avatar_url: "images/200x200.png",
        },
        {
          id: "5",
          name: "Robert",
          email: "robert@example.com",
          phone: "(27) 63 688 6444",
          avatar_url: "images/200x200.png",
        },
        {
          id: "6",
          name: "Tom",
          email: "thetom@example.com",
          phone: "(57) 63 688 6444",
          avatar_url: "images/200x200.png",
        },
        {
          id: "7",
          name: "Nolan",
          email: "Nolan@example.com",
          phone: "(27) 63 688 6444",
          avatar_url: "images/200x200.png",
        },
        {
          id: "8",
          name: "Adam",
          email: "Adam@example.com",
          phone: "(12) 22 888 4444",
          avatar_url: "images/200x200.png",
        },
        {
          id: "9",
          name: "Glen",
          email: "Glen@example.com",
          phone: "(74) 22 888 4444",
          avatar_url: "images/200x200.png",
        },
        {
          id: "10",
          name: "Edna",
          email: "Edna@example.com",
          phone: "(52) 33 888 4444",
          avatar_url: "images/200x200.png",
        },
        {
          id: "11",
          name: "Dianne",
          email: "dianne@example.com",
          phone: "(78) 33 888 4444",
          avatar_url: "images/200x200.png",
        },
        {
          id: "12",
          name: "Wilson",
          email: "wilson@example.com",
          phone: "(54) 63 688 6444",
          avatar_url: "images/200x200.png",
        },
        {
          id: "13",
          name: "Ross",
          email: "rose@example.com",
          phone: "(98) 63 688 6444",
          avatar_url: "images/200x200.png",
        },
        {
          id: "14",
          name: "Henry",
          email: "henry@example.com",
          phone: "(87) 63 688 6444",
          avatar_url: "images/200x200.png",
        },
        {
          id: "15",
          name: "Kerry",
          email: "kerry@example.com",
          phone: "(55) 63 688 6444",
          avatar_url: "images/200x200.png",
        },
      ],
      sort: true,
      search: true,
      pagination: {
        enabled: true,
        limit: 10,
      }
    },
    init() {
      this.table = new Gridjs.Grid(this.config).render(this.$root);
    },
    deleteItem() {
      this.$notification({ text: "Item remove action", variant: "warning" });
    },
    editItem() {
      this.$notification({ text: "Item edit action", variant: "info" });
    },
  };
}
