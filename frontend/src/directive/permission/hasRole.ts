 /**
 * v-hasRole 角色权限处理
 * Copyright (c) 2019 ruoyi
 */
import useUserStore from '@/store/modules/user'

export default {
  mounted(el: HTMLElement, binding: DirectiveBinding, vnode: any) {
    const { value } = binding
    const superAdmin = useUserStore().permissions.includes("*:*:*")
    const roles = useUserStore().roles

    if (value && value instanceof Array && value.length > 0) {
      const roleFlag = value

      const hasRole = superAdmin || roles.some((role: string) => {
        return roleFlag.includes(role)
      })

      if (!hasRole) {
        el.parentNode && el.parentNode.removeChild(el)
      }
    } else {
      throw new Error(`请设置角色权限标签值`)
    }
  }
}
