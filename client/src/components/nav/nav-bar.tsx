import React from 'react';
import { NavLink } from 'react-router-dom';
import { classNames } from '../../util/classnames.ts';
import styles from './nav-bar.module.scss';

export const NavBar: React.FC = () => {
    return (
        <nav className={styles.navBar}>
            <span className={styles.title}>Weekly Ad Recipes</span>
            <NavLink
                to="/"
                className={({ isActive }) => classNames(styles.link, isActive && styles.activeLink)}
                end
            >
                Home
            </NavLink>
            <NavLink
                to="/settings"
                className={({ isActive }) => classNames(styles.link, isActive && styles.activeLink)}
            >
                Settings
            </NavLink>
        </nav>
    );
};
